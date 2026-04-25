use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use sqlx::Row;
use tracing::error;
use url::Url;

use crate::{
    db::DbConnection,
    states::{
        AppState, InnerAppState,
        visitor_state::{self, SleepingState, VisitorId},
    },
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
        tx: &mut DbConnection,
    ) -> Option<i64> {
        let referrer_origin = self.referrer_origin.as_deref()?;
        if referrer_origin == state.tracked_origin || referrer_origin == state.base_origin {
            // Don't count the tracked domain or the domain of OxiTrack as a referrer domain.
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

        let referrer_row = sqlx::query("SELECT id FROM referrers WHERE domain = ? LIMIT 1")
            .bind(domain)
            .fetch_optional(&mut *tx)
            .await
            .ok()?;

        if let Some(row) = referrer_row {
            return Some(row.get("id"));
        }

        // Check that the referrer domain actually exists to prevent submitting random domains.
        state.http_client.get(url.clone()).send().await.ok()?;

        // There is a possible race condition here.
        // If two requests try to insert at the same time,
        // then only one insertion will be successful.
        // If the insertion fails because of the constraint, we will try to select.
        let inserted_row = sqlx::query(
            "INSERT INTO referrers(domain)
            VALUES (?)
            ON CONFLICT(domain) DO NOTHING
            RETURNING id",
        )
        .bind(domain)
        .fetch_optional(&mut *tx)
        .await
        .ok()?;

        if let Some(row) = inserted_row {
            return Some(row.get("id"));
        }

        // A concurrent request inserted first.
        sqlx::query("SELECT id FROM referrers WHERE domain = ? LIMIT 1")
            .bind(domain)
            .fetch_one(&mut *tx)
            .await
            .ok()
            .map(|row| row.get("id"))
    }
}

/// `/post-sleep/{visitor_id}` — always returns 200. Real errors are logged.
pub async fn get(
    State(state): AppState,
    Query(params): Query<Params>,
    Path(visitor_id): Path<VisitorId>,
) -> StatusCode {
    if let Err(err) = record(state, params, visitor_id).await {
        error!("post-sleep failed for visitor_id {visitor_id}: {err:#}");
    }
    StatusCode::OK
}

async fn record(
    state: &'static InnerAppState,
    params: Params,
    visitor_id: VisitorId,
) -> anyhow::Result<()> {
    let mut tx = state.pool.begin().await?;

    let Some(SleepingState {
        path_id,
        registered_at,
    }) = visitor_state::post_sleep(&mut tx, visitor_id, i64::from(state.min_delay_sec)).await?
    else {
        return Ok(());
    };

    let referrer_id = params.referrer_id(state, &mut tx).await;

    let visit_id: i64 = sqlx::query(
        "INSERT INTO visits(path_id, registered_at, referrer_id)
        VALUES (?, ?, ?)
        RETURNING id",
    )
    .bind(path_id)
    .bind(registered_at)
    .bind(referrer_id)
    .fetch_one(&mut *tx)
    .await?
    .get("id");

    visitor_state::post_visit_insertion(&mut tx, visitor_id, visit_id).await?;

    tx.commit().await?;

    Ok(())
}
