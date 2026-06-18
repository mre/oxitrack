use anyhow::{Context, Result};
use axum::{
    Router,
    http::header::{self, HeaderValue},
    routing::get,
};
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use std::net::SocketAddr;
use time::UtcOffset;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    services::ServeDir,
    set_header::SetResponseHeaderLayer,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing::Level;
use tracing_subscriber::{filter::LevelFilter, util::SubscriberInitExt};

use crate::{config::Config, handlers, states::InnerAppState};

static CONTENT_SECURITY_POLICY: HeaderValue = HeaderValue::from_static(
    "default-src 'self'; script-src 'self' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; connect-src 'self'; object-src 'none';",
);

fn load_config() -> Result<Config> {
    let path = std::env::var("OXITRACK_CONFIG_FILE").unwrap_or_else(|_| "config.toml".to_string());

    Figment::new()
        .merge(Toml::file(&path))
        .merge(
            Env::prefixed("OXITRACK_")
                .ignore(&["CONFIG_FILE"])
                .split("__"),
        )
        .extract()
        .with_context(|| format!("Could not load config from {path} (or OXITRACK_* env vars)"))
}

pub async fn app() -> Result<(Router, SocketAddr)> {
    let default_max_level = if cfg!(debug_assertions) {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };
    let max_level = std::env::var("RUST_LOG")
        .ok()
        .and_then(|v| match v.to_ascii_lowercase().as_str() {
            "off" => Some(LevelFilter::OFF),
            "error" => Some(LevelFilter::ERROR),
            "warn" => Some(LevelFilter::WARN),
            "info" => Some(LevelFilter::INFO),
            "debug" => Some(LevelFilter::DEBUG),
            "trace" => Some(LevelFilter::TRACE),
            _ => None,
        })
        .unwrap_or(default_max_level);
    tracing_subscriber::fmt::SubscriberBuilder::default()
        .with_max_level(max_level)
        .without_time()
        .finish()
        .try_init()
        .context("Failed to initialize the tracer!")?;
    let config = load_config()?;
    let utc_offset = UtcOffset::from_hms(config.utc_offset.hours, config.utc_offset.minutes, 0)
        .context("Invalid UTC offset configuration!")?;

    let socket_address = config.socket_address;

    let allowed_origin = config
        .tracked_origin
        .parse::<HeaderValue>()
        .context("Failed to parse the tracked origin!")?;

    let app_state = Box::leak(Box::new(InnerAppState::build(config, utc_offset).await?));

    let compression_layer = CompressionLayer::new().gzip(true);

    let count_js_router = Router::new()
        .route("/count.js", get(handlers::count_js::get))
        .layer(compression_layer.clone());

    let cors_router = Router::new()
        .route("/register", get(handlers::register::get))
        .route("/post-sleep/{visitor_id}", get(handlers::post_sleep::get))
        .route(
            "/page-left/{visitor_id}/{time_on_page_sec}",
            get(handlers::page_left::get),
        )
        .merge(count_js_router)
        .layer(CorsLayer::new().allow_origin(allowed_origin));

    let api_router = Router::new()
        .route("/history", get(handlers::api::history::get))
        .route("/counts", get(handlers::api::counts::get))
        .route("/count", get(handlers::api::count::get))
        .route("/live", get(handlers::api::live::get));

    let hx_router = Router::new()
        .route("/stats", get(handlers::dashboard::hx_stats::get))
        .route("/referrer", get(handlers::dashboard::hx_referrer::get))
        .route("/pages", get(handlers::dashboard::hx_filter::pages))
        .route("/referrers", get(handlers::dashboard::hx_filter::referrers))
        .layer(compression_layer.clone());

    let dashboard_router = Router::new()
        .route("/", get(handlers::dashboard::index::get))
        .route("/stats", get(handlers::dashboard::stats::get))
        .route("/referrer", get(handlers::dashboard::referrer::get))
        .layer(compression_layer);

    let static_service = ServeDir::new("static");

    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_request(())
        .on_response(())
        .on_body_chunk(())
        .on_eos(())
        .on_failure(());

    #[cfg(debug_assertions)]
    let trace_layer =
        trace_layer.on_request(tower_http::trace::DefaultOnRequest::new().level(Level::DEBUG));

    let csp_layer = SetResponseHeaderLayer::if_not_present(
        header::CONTENT_SECURITY_POLICY,
        CONTENT_SECURITY_POLICY.clone(),
    );

    let app = Router::new()
        .nest_service("/static", static_service)
        .route("/health", get(handlers::health::get))
        .merge(cors_router)
        .merge(dashboard_router)
        .nest("/hx", hx_router)
        .nest("/api", api_router)
        .layer(trace_layer)
        .layer(csp_layer)
        .with_state(app_state);

    Ok((app, socket_address))
}

#[cfg(test)]
mod tests {
    use super::app;
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
    };
    use figment::Jail;
    use mime::Mime;
    use tower::{Service, ServiceExt};

    struct Req {
        path: &'static str,
        status: StatusCode,
        mime: Option<Mime>,
        output: Option<&'static str>,
        /// Optional `(header_name, expected_value)` assertion. Used to verify
        /// htmx-driven endpoints set `HX-Push-Url` so the browser address bar
        /// stays in sync with the active filter without any client-side glue.
        header: Option<(&'static str, &'static str)>,
    }

    impl Req {
        fn new(path: &'static str) -> Self {
            Self {
                path,
                status: StatusCode::OK,
                mime: None,
                output: None,
                header: None,
            }
        }

        fn header(mut self, name: &'static str, value: &'static str) -> Self {
            self.header = Some((name, value));
            self
        }

        fn mime(mut self, mime: Mime) -> Self {
            self.mime = Some(mime);
            self
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn simple_requests() {
        let requests = [
            // register/post-sleep
            //
            // Session endpoints always return 200 — see the doc comments on
            // `post_sleep::get` / `page_left::get` for the rationale. Visitor
            // IDs are now random 53-bit integers persisted in the `sessions`
            // table, so we no longer assert a specific body here; the
            // `simple_requests_with_known_ids` flow is exercised separately
            // in the `visitor_state` unit tests and in end-to-end runs.
            Req::new("/post-sleep/0"),
            Req::new("/register?path=/").mime(mime::APPLICATION_JSON),
            Req::new("/register?path=/").mime(mime::APPLICATION_JSON),
            Req::new("/post-sleep/0"),
            Req::new("/post-sleep/1"),
            Req::new("/page-left/0/60"),
            // Dashboard
            Req::new("/").mime(mime::TEXT_HTML_UTF_8),
            Req::new("/?from=2026-03-24&to=2026-04-23").mime(mime::TEXT_HTML_UTF_8),
            // Stats
            Req::new("/stats?path=/").mime(mime::TEXT_HTML_UTF_8),
            Req::new("/stats?path=/&from=2026-03-24&to=2026-04-23").mime(mime::TEXT_HTML_UTF_8),
            Req::new("/stats?from=2026-03-24&to=2026-04-23&path=/").mime(mime::TEXT_HTML_UTF_8),
            // htmx partial: must echo the canonical public URL via HX-Push-Url
            // so the browser address bar updates without any client-side JS.
            Req::new("/hx/stats?from=2026-03-24&to=2026-04-23")
                .header("hx-push-url", "/?from=2026-03-24&to=2026-04-23"),
            // `/` in `path` is percent-encoded by `serde_urlencoded`, which is
            // valid per RFC 3986 and decoded transparently by axum on the next request.
            Req::new("/hx/stats?path=/&from=2026-03-24&to=2026-04-23").header(
                "hx-push-url",
                "/stats?from=2026-03-24&to=2026-04-23&path=%2F",
            ),
            Req::new("/hx/stats").header("hx-push-url", "/"),
            // API
            Req::new("/api/counts").mime(mime::APPLICATION_JSON),
            Req::new("/api/history?path=/").mime(mime::APPLICATION_JSON),
        ];

        Jail::expect_with(|jail| {
            jail.set_env("OXITRACK_CONFIG_FILE", "config.toml");

            jail.create_file(
                "config.toml",
                r#"
                socket_address = "127.0.0.1:8080"
                base_url = "http://127.0.0.1:8080"
                tracked_origin = "https://mo8it.com"
                min_delay_secs = 0

                [db]
                path = ":memory:"
                "#,
            )?;

            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let (mut app, ..) = app().await.unwrap();

                    // Seed two real visits via the random-id round-trip so
                    // `/stats?path=/` below has data to render.
                    let mut call = async |uri: String| {
                        let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
                        let resp = app
                            .as_service()
                            .ready()
                            .await
                            .unwrap()
                            .call(req)
                            .await
                            .unwrap();
                        assert_eq!(resp.status(), StatusCode::OK);
                        to_bytes(resp.into_body(), 1 << 10).await.unwrap()
                    };
                    for _ in 0..2 {
                        let id = call("/register?path=/".into()).await;
                        let id = std::str::from_utf8(&id).unwrap().trim();
                        call(format!("/post-sleep/{id}")).await;
                    }

                    for req in requests {
                        let request = Request::builder()
                            .uri(req.path)
                            .body(Body::empty())
                            .unwrap();
                        let response = app
                            .as_service()
                            .ready()
                            .await
                            .unwrap()
                            .call(request)
                            .await
                            .unwrap();

                        assert_eq!(response.status(), req.status, "path={}", req.path);

                        if let Some(mime) = req.mime {
                            assert_eq!(
                                response
                                    .headers()
                                    .get(header::CONTENT_TYPE)
                                    .unwrap()
                                    .to_str()
                                    .unwrap(),
                                mime.as_ref(),
                                "path={}",
                                req.path,
                            );
                        }

                        if let Some((name, expected)) = req.header {
                            let actual = response
                                .headers()
                                .get(name)
                                .unwrap_or_else(|| {
                                    panic!("path={}: missing header {name}", req.path)
                                })
                                .to_str()
                                .unwrap();
                            assert_eq!(actual, expected, "path={} header={name}", req.path);
                        }

                        if let Some(output) = req.output {
                            assert_eq!(
                                to_bytes(response.into_body(), 1 << 10).await.unwrap(),
                                output,
                                "path={}",
                                req.path,
                            );
                        }
                    }
                });

            Ok(())
        });
    }
}
