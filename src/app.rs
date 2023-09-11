use axum::{http::HeaderValue, routing::get, Router};
use oxi_axum_helpers::{static_handler, InitErr, InitErrCtx, PreTracer};
use rust_embed::RustEmbed;
use std::{net::SocketAddr, sync::Arc};
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    trace::{DefaultMakeSpan, DefaultOnRequest, TraceLayer},
};
use tracing::Level;

use crate::{
    config::Config,
    handlers::{self, states::AppState},
};

pub const DATA_DIR_ENV_VAR: &str = "OXITRAFFIC_DATA_DIR";

#[derive(RustEmbed)]
#[folder = "static/"]
struct Static;

pub async fn app() -> Result<(Router, SocketAddr), InitErr> {
    let PreTracer {
        config, utc_offset, ..
    } = PreTracer::<Config>::init(DATA_DIR_ENV_VAR, "oxitraffic");

    let socket_address = config.socket_address;

    let allowed_origin = config
        .tracked_origin
        .parse::<HeaderValue>()
        .init_ctx("Failed to parse the tracked origin!")?;

    let app_state = Arc::new(AppState::build(config, utc_offset).await?);

    let compression_layer = CompressionLayer::new().gzip(true);

    let counting_router = Router::new()
        .route("/register", get(handlers::register::get))
        .route(
            "/post-sleep/:registration_id",
            get(handlers::post_sleep::get),
        )
        .layer(CorsLayer::new().allow_origin(allowed_origin));

    let api_router = Router::new()
        .route("/history", get(handlers::api::history::get))
        .route("/counts", get(handlers::api::counts::get))
        .layer(compression_layer.clone());

    let dashboard_router = Router::new()
        .route("/", get(handlers::dashboard::index::get))
        .route("/stats", get(handlers::dashboard::stats::get))
        .layer(compression_layer);

    let app = Router::new()
        .route("/static/:file", get(static_handler::handler::<Static>))
        .merge(counting_router)
        .nest("/api", api_router)
        .merge(dashboard_router)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(())
                .on_body_chunk(())
                .on_eos(())
                .on_failure(()),
        )
        .with_state(app_state);

    Ok((app, socket_address))
}

#[cfg(test)]
mod tests {
    use super::{app, Static, DATA_DIR_ENV_VAR};
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };
    use figment::Jail;
    use mime::Mime;
    use tower::{Service, ServiceExt};
    use typed_builder::TypedBuilder;

    #[derive(TypedBuilder)]
    struct Req {
        path: &'static str,
        #[builder(default, setter(strip_option))]
        mime: Option<Mime>,
        #[builder(default=StatusCode::OK)]
        status: StatusCode,
        #[builder(default, setter(strip_option))]
        output: Option<&'static str>,
    }

    #[test]
    fn static_files() {
        Static::get("main.css").unwrap();
    }

    fn requests() -> [Req; 13] {
        [
            // Static files
            Req::builder()
                .path("/static/main.css")
                .mime(mime::TEXT_CSS)
                .build(),
            Req::builder()
                .path("/static/main.css?v=foo")
                .mime(mime::TEXT_CSS)
                .build(),
            // register/post-sleep
            Req::builder()
                .path("/post-sleep/0")
                .status(StatusCode::BAD_REQUEST)
                .build(),
            Req::builder()
                .path("/register?path=/")
                .mime(mime::APPLICATION_JSON)
                .output("0")
                .build(),
            Req::builder()
                .path("/register?path=/")
                .mime(mime::APPLICATION_JSON)
                .output("1")
                .build(),
            Req::builder().path("/post-sleep/0").build(),
            Req::builder().path("/post-sleep/1").build(),
            Req::builder()
                .path("/post-sleep/0")
                .status(StatusCode::BAD_REQUEST)
                .build(),
            Req::builder()
                .path("/post-sleep/1")
                .status(StatusCode::BAD_REQUEST)
                .build(),
            // Dashboard
            Req::builder().path("/").mime(mime::TEXT_HTML_UTF_8).build(),
            // Stats
            Req::builder()
                .path("/stats?path=/")
                .mime(mime::TEXT_HTML_UTF_8)
                .build(),
            // API
            Req::builder()
                .path("/api/counts")
                .mime(mime::APPLICATION_JSON)
                .build(),
            Req::builder()
                .path("/api/history?path=/")
                .mime(mime::APPLICATION_JSON)
                .build(),
        ]
    }

    #[test]
    fn simple_requests() {
        Jail::expect_with(|jail| {
            jail.set_env(DATA_DIR_ENV_VAR, ".");

            jail.create_file(
                "config.yaml",
                r#"
                    tracked_origin: https://mo8it.com

                    min_delay_secs: 0

                    db:
                      host: 127.0.0.1
                      port: 5432
                      username: postgres
                      password: CHANGE_ME
                      database: postgres
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
                        let response = app.ready().await.unwrap().call(request).await.unwrap();

                        assert_eq!(response.status(), req.status, "path={}", req.path);

                        if let Some(mime) = req.mime {
                            assert_eq!(
                                response
                                    .headers()
                                    .get(http::header::CONTENT_TYPE)
                                    .unwrap()
                                    .to_str()
                                    .unwrap(),
                                mime.as_ref(),
                                "path={}",
                                req.path
                            );
                        }

                        if let Some(output) = req.output {
                            assert_eq!(
                                hyper::body::to_bytes(response.into_body()).await.unwrap(),
                                output,
                                "path={}",
                                req.path
                            );
                        }
                    }
                });

            Ok(())
        });
    }
}
