use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
};
use oxi_axum_helpers::{RespErr, RespErrCtx, RespErrExt, Status};
use serde::Deserialize;
use url::Url;

use crate::states::{
    visitor_state::{SleepingState, VisitorId},
    AppState,
};

const MAX_DOMAIN_LEN: usize = 255;

#[derive(Deserialize)]
pub struct Params {
    referrer_origin: Option<String>,
}

impl Params {
    fn referrer_origin(&self, tracked_origin: &str) -> Option<Url> {
        let referrer_origin = self.referrer_origin.as_ref()?;

        if referrer_origin.starts_with(tracked_origin) {
            // Don't count the tracked domain as a referrer domain.
            return None;
        }

        Url::parse(referrer_origin).ok()
    }
}

fn referrer_domain(url: &Url) -> Option<&str> {
    if url.scheme() != "https" {
        return None;
    }

    let domain = url.domain()?;

    if domain.len() > MAX_DOMAIN_LEN || !domain.contains('.') {
        return None;
    }

    Some(domain)
}

pub async fn get(
    State(state): AppState,
    Query(params): Query<Params>,
    Path(visitor_id): Path<VisitorId>,
) -> Result<StatusCode, RespErr> {
    let SleepingState {
        path_id,
        registered_at,
    } = state
        .visitor_states
        .post_sleep(visitor_id)
        .ctx(Status::BadRequest)
        .user_msg("The visitor ID is invalid or has expired!")?;

    let referrer_origin = params.referrer_origin(&state.tracked_origin);
    let referrer_domain = referrer_origin.as_ref().and_then(referrer_domain);

    let visit_id = if let Some(referrer_domain) = referrer_domain {
        let mut tx = state
            .db
            .begin()
            .await
            .ctx(Status::Internal)
            .err_msg("Failed to begin a transaction!")?;

        // Try to insert if the referrer doesn't already exist.
        let referrer_id = sqlx::query!(
            "INSERT INTO referrers(domain) VALUES ($1)
            ON CONFLICT DO NOTHING
            RETURNING id",
            referrer_domain
        )
        .fetch_optional(&mut *tx)
        .await
        .ctx(Status::Internal)
        .err_msg("Failed to insert a referrer!")?;

        let referrer_id = if let Some(id) = referrer_id {
            id.id
        } else {
            // Insertion had a conflict, therefore the referrer must already exist.
            sqlx::query!(
                "SELECT id FROM referrers
                WHERE domain = $1",
                referrer_domain,
            )
            .fetch_one(&mut *tx)
            .await
            .ctx(Status::Internal)
            .err_msg("Referrer not found although its insertion had a conflict!")?
            .id
        };

        let visit_id = sqlx::query!(
            "INSERT INTO visits(path_id, registered_at, referrer_id) VALUES ($1, $2, $3)
            RETURNING id",
            path_id,
            registered_at,
            referrer_id,
        )
        .fetch_one(&mut *tx)
        .await
        .ctx(Status::Internal)
        .err_msg(|| format!("Failed to insert a visit for path_id {path_id}!"))?
        .id;

        tx.commit()
            .await
            .ctx(Status::Internal)
            .err_msg("Failed to commit the post sleep transaction with referrer.")?;

        visit_id
    } else {
        sqlx::query!(
            "INSERT INTO visits(path_id, registered_at) VALUES ($1, $2)
            RETURNING id",
            path_id,
            registered_at,
        )
        .fetch_one(&*state.db)
        .await
        .ctx(Status::Internal)
        .err_msg(|| format!("Failed to insert a visit for path_id {path_id}!"))?
        .id
    };

    state
        .visitor_states
        .post_visit_insertion(visitor_id, visit_id);

    Ok(StatusCode::OK)
}
