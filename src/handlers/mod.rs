pub mod states;

use axum::{
    body::Full,
    extract::{Path, State},
    http::header,
    response::{IntoResponse, Response},
    Json,
};
use futures::TryStreamExt;
use resp_err::{RespErr, RespErrCtx, Status};
use std::sync::Arc;
use tracing::instrument;

use self::states::AppState;

pub type AppStateT = State<Arc<AppState>>;

struct Id {
    id: i64,
}

#[instrument(skip_all)]
pub async fn call(State(state): AppStateT, Path(path): Path<String>) -> Result<Response, RespErr> {
    let path_id = sqlx::query_as!(Id, "SELECT id FROM paths WHERE path = $1", path)
        .fetch_optional(&*state.db)
        .await
        .ctx(Status::Internal)?;

    let path_id = match path_id {
        Some(id) => id.id,
        None => {
            let status = reqwest::get(format!("{}/{path}", state.tracked_base_url))
                .await
                .ctx(Status::Internal)?
                .status();

            if status == 200 {
                sqlx::query_as!(Id, "INSERT INTO paths(path) VALUES ($1) RETURNING id", path)
                    .fetch_one(&*state.db)
                    .await
                    .ctx(Status::Internal)?
                    .id
            } else {
                return Err(RespErr::new(Status::NotFound));
            }
        }
    };

    sqlx::query!("INSERT INTO calls(path_id) VALUES ($1)", path_id)
        .execute(&*state.db)
        .await
        .ctx(Status::Internal)?;

    let headers = [(header::CONTENT_TYPE, &state.mime)];
    let body = Full::from(state.file_content);

    Ok((headers, body).into_response())
}

struct TimeStamp {
    timestamp: Option<String>,
}

#[instrument(skip_all)]
pub async fn data(
    State(state): AppStateT,
    Path(path): Path<String>,
) -> Result<Json<Vec<String>>, RespErr> {
    let path_id = sqlx::query_as!(Id, "SELECT id FROM paths WHERE path = $1", path)
        .fetch_one(&*state.db)
        .await
        .ctx(Status::NotFound)?
        .id;

    let timestamps = sqlx::query_as!(
        TimeStamp,
        "SELECT timestamp::text FROM calls WHERE path_id = $1 ORDER BY timestamp",
        path_id,
    )
    .fetch(&*state.db)
    .try_filter_map(|row| async move { Ok(row.timestamp) })
    .try_collect()
    .await
    .ctx(Status::BadRequest)?;

    Ok(Json(timestamps))
}
