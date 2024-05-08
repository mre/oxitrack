use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
};
use axum_ctx::*;
use serde::Deserialize;
use sqlx::PgConnection;
use url::Url;

use crate::states::{
    visitor_state::{SleepingState, VisitorId},
    AppState, InnerAppState,
};

const MAX_DOMAIN_LEN: usize = 255;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Params {
    referrer_origin: Option<String>,
}

impl Params {
    async fn referrer_id(
        &self,
        state: &'static InnerAppState,
        tx: &mut PgConnection,
    ) -> Option<i64> {
        let referrer_origin = self.referrer_origin.as_deref()?;
        if referrer_origin == state.tracked_origin || referrer_origin == state.base_origin {
            // Don't count the tracked domain or the domain of OxiTraffic as a referrer domain.
            return None;
        }

        let url = Url::parse(referrer_origin).ok()?;
        if url.scheme() != "https" {
            return None;
        }

        let domain = url.domain()?;
        if domain.len() > MAX_DOMAIN_LEN || !domain.contains('.') {
            return None;
        }

        let referrer_row = sqlx::query!(
            "SELECT id FROM referrers
            WHERE domain = $1
            LIMIT 1",
            domain,
        )
        .fetch_optional(&mut *tx)
        .await
        .ok()?;

        if let Some(row) = referrer_row {
            return Some(row.id);
        }

        // Check that the referrer domain actually exists to prevent submitting random domains.
        state.http_client.get(url.clone()).send().await.ok()?;

        // There is a possible race condition here.
        // If two requests try to insert at the same time,
        // then only one insertion will be succussful.
        // If the insertion fails because of the constraint, we will try to select.
        let inserted_row = sqlx::query!(
            "INSERT INTO referrers(domain)
            VALUES ($1)
            ON CONFLICT ON CONSTRAINT unique_domain DO NOTHING
            RETURNING id",
            domain,
        )
        .fetch_optional(&mut *tx)
        .await
        .ok()?;

        if let Some(row) = inserted_row {
            return Some(row.id);
        }

        // A concurrent request inserted first.
        sqlx::query!(
            "SELECT id FROM referrers
            WHERE domain = $1
            LIMIT 1",
            domain,
        )
        .fetch_one(&mut *tx)
        .await
        .ok()
        .map(|row| row.id)
    }
}

pub async fn get(
    State(state): AppState,
    Query(params): Query<Params>,
    Path(visitor_id): Path<VisitorId>,
) -> RespResult<StatusCode> {
    let SleepingState {
        path_id,
        registered_at,
    } = state
        .visitor_states
        .post_sleep(visitor_id)
        .ctx(StatusCode::BAD_REQUEST)
        .user_msg("The visitor ID is invalid or has expired!")?;

    let mut tx = state
        .pool
        .begin()
        .await
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg("Failed to begin a transaction!")?;

    let referrer_id = params.referrer_id(state, &mut tx).await;

    let visit_id = sqlx::query!(
        "INSERT INTO visits(path_id, registered_at, referrer_id)
        VALUES ($1, $2, $3)
        RETURNING id",
        path_id,
        registered_at,
        referrer_id,
    )
    .fetch_one(&mut *tx)
    .await
    .ctx(StatusCode::INTERNAL_SERVER_ERROR)
    .log_msg(|| format!("Failed to insert a visit for the path_id {path_id}!"))?
    .id;

    tx.commit()
        .await
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg(|| {
            format!("Failed to commit the post-sleep transaction for the path_id {path_id}!")
        })?;

    state
        .visitor_states
        .post_visit_insertion(visitor_id, visit_id);

    Ok(StatusCode::OK)
}
