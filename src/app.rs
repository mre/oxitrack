use anyhow::{Context, Result};
use axum::{http::header::HeaderValue, routing::get, Router};
use oxi_axum_helpers::{init_config, init_tracer, static_router};
use std::net::SocketAddr;
use time::UtcOffset;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing::Level;

use crate::{config::Config, handlers, states::InnerAppState};

pub async fn app() -> Result<(Router, SocketAddr)> {
    init_tracer()?;
    let config = init_config::<Config>("oxitraffic")?;
    let utc_offset = UtcOffset::from_hms(config.utc_offset.hours, config.utc_offset.minutes, 0)
        .context("Invalid UTC offset configuration!")?;

    let socket_address = config.socket_address;

    let allowed_origin = config
        .tracked_origin
        .parse::<HeaderValue>()
        .context("Failed to parse the tracked origin!")?;

    let app_state = Box::leak(Box::new(InnerAppState::build(config, utc_offset).await?));

    let compression_layer = CompressionLayer::new().gzip(true);

    let chart_data_router = Router::new()
        .route(
            "/last-60-days",
            get(handlers::stats_data::last_60_days::get),
        )
        .route("/last-2-days", get(handlers::stats_data::last_2_days::get))
        .route("/all-time", get(handlers::stats_data::all_time::get))
        .layer(compression_layer.clone());

    let count_js_router = Router::new()
        .route("/count.js", get(handlers::count_js::get))
        .layer(compression_layer.clone());

    let cors_router = Router::new()
        .route("/register", get(handlers::register::get))
        .route("/post-sleep/:visitor_id", get(handlers::post_sleep::get))
        .route(
            "/page-left/:visitor_id/:time_on_page_sec",
            get(handlers::page_left::get),
        )
        .merge(count_js_router)
        .nest("/stats-data", chart_data_router)
        .layer(CorsLayer::new().allow_origin(allowed_origin));

    let api_router = Router::new()
        .route("/history", get(handlers::api::history::get))
        .route("/counts", get(handlers::api::counts::get))
        .route("/count", get(handlers::api::count::get))
        .layer(compression_layer.clone());

    let dashboard_router = Router::new()
        .route("/", get(handlers::dashboard::index::get))
        .route("/stats", get(handlers::dashboard::stats::get))
        .layer(compression_layer);

    let static_router = static_router!(
        "../static",
        ("main.css", "text/css"),
        ("stats.js", "application/javascript"),
        ("stats.js.map", "application/json"),
        ("logo.svg", "image/svg+xml"),
    );

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

    let app = Router::new()
        .merge(static_router)
        .merge(cors_router)
        .merge(dashboard_router)
        .nest("/api", api_router)
        .layer(trace_layer)
        .with_state(app_state);

    Ok((app, socket_address))
}

#[cfg(test)]
mod tests {
    use super::app;
    use axum::{
        body::{to_bytes, Body},
        http::{header, Request, StatusCode},
    };
    use figment::Jail;
    use mime::Mime;
    use tower::{Service, ServiceExt};

    struct Req {
        path: &'static str,
        status: StatusCode,
        mime: Option<Mime>,
        output: Option<&'static str>,
    }

    impl Req {
        fn new(path: &'static str) -> Self {
            Req {
                path,
                status: StatusCode::OK,
                mime: None,
                output: None,
            }
        }

        fn status(mut self, status: StatusCode) -> Self {
            self.status = status;
            self
        }

        fn mime(mut self, mime: Mime) -> Self {
            self.mime = Some(mime);
            self
        }

        fn output(mut self, output: &'static str) -> Self {
            self.output = Some(output);
            self
        }
    }

    fn requests() -> [Req; 13] {
        [
            // Static files
            Req::new("/static/main.css").mime(mime::TEXT_CSS_UTF_8),
            Req::new("/static/main.css?v=foo").mime(mime::TEXT_CSS_UTF_8),
            // register/post-sleep
            Req::new("/post-sleep/0").status(StatusCode::BAD_REQUEST),
            Req::new("/register?path=/")
                .mime(mime::APPLICATION_JSON)
                .output("0"),
            Req::new("/register?path=/")
                .mime(mime::APPLICATION_JSON)
                .output("1"),
            Req::new("/post-sleep/0"),
            Req::new("/post-sleep/1"),
            Req::new("/post-sleep/0").status(StatusCode::BAD_REQUEST),
            Req::new("/post-sleep/1").status(StatusCode::BAD_REQUEST),
            // Dashboard
            Req::new("/").mime(mime::TEXT_HTML_UTF_8),
            // Stats
            Req::new("/stats?path=/").mime(mime::TEXT_HTML_UTF_8),
            // API
            Req::new("/api/counts").mime(mime::APPLICATION_JSON),
            Req::new("/api/history?path=/").mime(mime::APPLICATION_JSON),
        ]
    }

    #[test]
    fn simple_requests() {
        Jail::expect_with(|jail| {
            jail.set_env("OXITRAFFIC_CONFIG_FILE", "config.toml");

            jail.create_file(
                "config.toml",
                r#"
                socket_address = "127.0.0.1:8080"
                base_url = "http://127.0.0.1:8080"
                tracked_origin = "https://mo8it.com"

                min_delay_secs = 0

                [db]
                host = "127.0.0.1"
                port = 5432
                username = "postgres"
                password = "CHANGE_ME"
                database = "postgres"
                "#,
            )?;

            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let (mut app, ..) = app().await.unwrap();

                    for req in requests() {
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
