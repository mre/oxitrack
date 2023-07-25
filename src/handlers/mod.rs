pub mod api;
pub mod states;

use axum::{
    body::Full,
    extract::{ConnectInfo, Path, State},
    http::header,
    response::{IntoResponse, Response},
};
use reqwest::StatusCode;
use resp_err::{RespErr, RespErrCtx, Status};
use std::{net::SocketAddr, sync::Arc};
use tracing::instrument;

use crate::db::Id;

use self::states::AppState;

pub type AppStateT = State<Arc<AppState>>;

fn call_reponse(state: Arc<AppState>) -> Response {
    let headers = [(header::CONTENT_TYPE, &state.mime)];
    let body = Full::from(state.file_content);

    (headers, body).into_response()
}

#[instrument(skip_all)]
pub async fn call(
    State(state): AppStateT,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(path): Path<String>,
) -> Result<Response, RespErr> {
    let path_id = sqlx::query_as!(Id, "SELECT id FROM paths WHERE path = $1", path)
        .fetch_optional(&*state.db)
        .await
        .ctx(Status::Internal)?;

    let path_id = match path_id {
        Some(Id { id }) => {
            let new_call = state.anti_spam.lock().unwrap().insert((id, addr.ip()));

            if !new_call {
                return Ok(call_reponse(state));
            }

            id
        }
        None => {
            let status = reqwest::get(format!("{}/{path}", state.tracked_base_url))
                .await
                .ctx(Status::Internal)?
                .status();

            if status != StatusCode::OK {
                return Err(RespErr::new(Status::NotFound));
            }

            let id = sqlx::query_as!(Id, "INSERT INTO paths(path) VALUES ($1) RETURNING id", path)
                .fetch_one(&*state.db)
                .await
                .ctx(Status::Internal)?
                .id;

            state.anti_spam.lock().unwrap().insert((id, addr.ip()));

            id
        }
    };

    sqlx::query!("INSERT INTO calls(path_id) VALUES ($1)", path_id)
        .execute(&*state.db)
        .await
        .ctx(Status::Internal)?;

    Ok(call_reponse(state))
}
