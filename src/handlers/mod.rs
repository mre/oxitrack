pub mod api;
mod base_template;
pub mod dashboard;
pub mod states;

use axum::{
    body::Full,
    extract::{ConnectInfo, Path, State},
    http::header,
    response::{IntoResponse, Response},
};
use oxi_axum_helpers::{RespErr, RespErrCtx, RespErrExt, Status};
use reqwest::StatusCode;
use std::{net::SocketAddr, sync::Arc};
use tracing::{info, instrument};

use crate::db::Id;

use self::states::AppState;

pub type AppStateT = State<Arc<AppState>>;

fn call_response(state: Arc<AppState>) -> Response {
    let headers = [(header::CONTENT_TYPE, &state.mime)];
    let body = Full::from(state.file_content);

    (headers, body).into_response()
}

async fn handle_call(
    state: Arc<AppState>,
    addr: SocketAddr,
    path: &str,
) -> Result<Response, RespErr> {
    let path_id = sqlx::query_as!(Id, "SELECT id FROM paths WHERE path = $1", path)
        .fetch_optional(&*state.db)
        .await
        .ctx(Status::Internal)
        .err_msg("Failed to run path query!")?;

    let path_id = match path_id {
        Some(Id { id }) => {
            let new_call = state.anti_spam.lock().unwrap().insert((id, addr.ip()));

            if !new_call {
                return Ok(call_response(state));
            }

            id
        }
        None => {
            let status = reqwest::get(format!("{}/{path}", state.tracked_base_url))
                .await
                .ctx(Status::Internal)
                .err_msg("Failed to look up the path on the tracked website!")?
                .status();

            if status != StatusCode::OK {
                return Err(
                    RespErr::new(Status::NotFound).err_msg("Path not found on tracked website!")
                );
            }

            let id = sqlx::query_as!(Id, "INSERT INTO paths(path) VALUES ($1) RETURNING id", path)
                .fetch_one(&*state.db)
                .await
                .ctx(Status::Internal)
                .err_msg("Failed to insert path!")?
                .id;

            state.anti_spam.lock().unwrap().insert((id, addr.ip()));

            id
        }
    };

    sqlx::query!("INSERT INTO calls(path_id) VALUES ($1)", path_id)
        .execute(&*state.db)
        .await
        .ctx(Status::Internal)
        .err_msg("Failed to insert call!")?;

    Ok(call_response(state))
}

#[instrument(skip_all)]
pub async fn call_index(
    State(state): AppStateT,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Response, RespErr> {
    info!("Call: INDEX");

    let path = "";

    handle_call(state, addr, path).await
}

#[instrument(skip_all)]
pub async fn call(
    State(state): AppStateT,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(path): Path<String>,
) -> Result<Response, RespErr> {
    info!("Call: {path}");

    let path = path.trim_end_matches('/');

    handle_call(state, addr, path).await
}
