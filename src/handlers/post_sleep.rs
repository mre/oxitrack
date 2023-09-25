use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
};
use oxi_axum_helpers::{RespErr, RespErrCtx, RespErrExt, Status};
use serde::Deserialize;
use url::Url;

use crate::{
    db::Id,
    states::{sleeping_hotel::SleepingHotelInd, AppState},
};

#[derive(Deserialize)]
pub struct Params {
    referrer: Option<String>,
}

impl Params {
    fn referrer(&self, tracked_origin: &str) -> Option<Url> {
        let referrer = self.referrer.as_ref()?;

        if referrer.starts_with(tracked_origin) {
            // Don't count the tracked domain as a referrer domain.
            return None;
        }

        Url::parse(referrer).ok()
    }
}

pub async fn get(
    State(state): AppState,
    Query(params): Query<Params>,
    Path(registration_id): Path<SleepingHotelInd>,
) -> Result<StatusCode, RespErr> {
    let path_id = state
        .sleeping_hotel
        .lock()
        .unwrap()
        .wake_up(registration_id)
        .ctx(Status::BadRequest)
        .user_msg("The registered ID is invalid or has expired!")?;

    let referrer = params.referrer(&state.tracked_origin);
    let referrer_domain = referrer.as_ref().and_then(|r| r.domain());

    if let Some(referrer_domain) = referrer_domain {
        let mut tx = state
            .db
            .begin()
            .await
            .ctx(Status::Internal)
            .err_msg("Failed to begin a transaction!")?;

        // Try to insert if the referrer doesn't already exist.
        let referrer_id = sqlx::query_as!(
            Id,
            "INSERT INTO referrers(domain) VALUES ($1)
            ON CONFLICT DO NOTHING
            RETURNING id",
            referrer_domain
        )
        .fetch_optional(&mut *tx)
        .await
        .ctx(Status::Internal)
        .err_msg("Failed to insert a referrer!")?;

        let referrer_id = if let Some(Id { id }) = referrer_id {
            id
        } else {
            // Insertion had a conflict, therefore the referrer must already exist.
            sqlx::query_as!(
                Id,
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

        sqlx::query!(
            "INSERT INTO visits(path_id, referrer_id) VALUES ($1, $2)",
            path_id,
            referrer_id,
        )
        .execute(&mut *tx)
        .await
        .ctx(Status::Internal)
        .err_msg_lz(|| format!("Failed to insert a visit for path_id {path_id}!"))?;

        tx.commit()
            .await
            .ctx(Status::Internal)
            .err_msg("Failed to commit the post sleep transaction with referrer.")?;
    } else {
        sqlx::query!("INSERT INTO visits(path_id) VALUES ($1)", path_id,)
            .execute(&*state.db)
            .await
            .ctx(Status::Internal)
            .err_msg_lz(|| format!("Failed to insert call for path_id {path_id}!"))?;
    }

    Ok(StatusCode::OK)
}
