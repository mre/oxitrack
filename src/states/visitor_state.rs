//! Persistent visitor session storage backed by the `sessions` `SQLite` table.
//!
//! Lifecycle:
//! 1. `/register`   → [`register`] inserts a row with `visit_id IS NULL`.
//! 2. `/post-sleep` → [`post_sleep`] reads it, then [`post_visit_insertion`]
//!    links the freshly inserted `visits.id`.
//! 3. `/page-left`  → [`page_left`] returns `visit_id` and deletes the row.
//!
//! Missing / expired / already-consumed sessions return `Ok(None)` so callers
//! can log-and-200 instead of 4xx-ing the user.

use rand::Rng;
use time::OffsetDateTime;

use crate::db::{DbConnection, DbPool};

pub type VisitorId = i64;
pub type VisitId = i64;
pub type PathId = i64;

/// Upper bound (exclusive) for minted [`VisitorId`]s. 2^53 is the largest
/// integer a JS `Number` can hold losslessly, so ids round-trip through the
/// browser's JSON parser without precision loss.
const VISITOR_ID_UPPER_EXCL: i64 = 1_i64 << 53;
const REGISTER_MAX_RETRIES: u8 = 5;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SleepingState {
    pub path_id: PathId,
    pub registered_at: OffsetDateTime,
}

/// Insert a new session and return its id. Retries on the vanishingly unlikely
/// PK collision. The insert also trips the `sessions_ttl` trigger.
pub async fn register(
    conn: &mut DbConnection,
    sleeping: &SleepingState,
) -> Result<VisitorId, sqlx::Error> {
    for _ in 0..REGISTER_MAX_RETRIES {
        let id = rand::rng().random_range(1..VISITOR_ID_UPPER_EXCL);

        let res = sqlx::query(
            "INSERT INTO sessions(visitor_id, path_id, registered_at)
             VALUES (?, ?, ?)
             ON CONFLICT(visitor_id) DO NOTHING",
        )
        .bind(id)
        .bind(sleeping.path_id)
        .bind(sleeping.registered_at)
        .execute(&mut *conn)
        .await?;

        if res.rows_affected() > 0 {
            return Ok(id);
        }
    }

    Err(sqlx::Error::Protocol(format!(
        "Failed to mint a unique visitor_id after {REGISTER_MAX_RETRIES} attempts"
    )))
}

/// Returns the sleeping state if the session exists, has not been promoted yet,
/// and has waited at least `min_secs`. The row is left in place; the caller
/// promotes it via [`post_visit_insertion`] after the `visits` INSERT commits.
pub async fn post_sleep(
    conn: &mut DbConnection,
    visitor_id: VisitorId,
    min_secs: i64,
) -> Result<Option<SleepingState>, sqlx::Error> {
    let row: Option<(PathId, OffsetDateTime, Option<VisitId>)> = sqlx::query_as(
        "SELECT path_id, registered_at, visit_id FROM sessions WHERE visitor_id = ?",
    )
    .bind(visitor_id)
    .fetch_optional(&mut *conn)
    .await?;

    let Some((path_id, registered_at, None)) = row else {
        return Ok(None);
    };

    if (OffsetDateTime::now_utc() - registered_at).whole_seconds() < min_secs {
        return Ok(None);
    }

    Ok(Some(SleepingState {
        path_id,
        registered_at,
    }))
}

/// Link a sleeping session to its freshly inserted `visits.id`.
pub async fn post_visit_insertion(
    conn: &mut DbConnection,
    visitor_id: VisitorId,
    visit_id: VisitId,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE sessions SET visit_id = ? WHERE visitor_id = ?")
        .bind(visit_id)
        .bind(visitor_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Delete the session and return its `visit_id` if it had been promoted.
pub async fn page_left(
    conn: &mut DbConnection,
    visitor_id: VisitorId,
) -> Result<Option<VisitId>, sqlx::Error> {
    let row: Option<(Option<VisitId>,)> =
        sqlx::query_as("DELETE FROM sessions WHERE visitor_id = ? RETURNING visit_id")
            .bind(visitor_id)
            .fetch_optional(&mut *conn)
            .await?;

    Ok(row.and_then(|(visit_id,)| visit_id))
}

/// How recent a session's `registered_at` must be to be considered "live".
///
/// `/page-left` is fired from `beforeunload`, which is unreliable (mobile
/// browsers, crashes, lost network, etc.), so the `sessions` table inevitably
/// accumulates orphaned rows that the 24h `sessions_ttl` trigger has not yet
/// swept. Counting all rows would massively over-report the number of people
/// currently on the site, so we bound it to a short recent window.
const LIVE_WINDOW_SECS: i64 = 5 * 60;

/// Number of live sessions: rows registered within the last
/// [`LIVE_WINDOW_SECS`] seconds and not yet `/page-left`. Older rows are
/// considered stale and ignored regardless of whether the TTL trigger has
/// swept them yet.
pub async fn live_count(pool: &DbPool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM sessions
         WHERE datetime(registered_at) >= datetime('now', ?)",
    )
    .bind(format!("-{LIVE_WINDOW_SECS} seconds"))
    .fetch_one(pool)
    .await
}

/// Distinct `path_id`s with at least one live session, using the same
/// recency window as [`live_count`].
pub async fn live_path_ids(pool: &DbPool) -> Result<Vec<PathId>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT DISTINCT path_id FROM sessions
         WHERE datetime(registered_at) >= datetime('now', ?)",
    )
    .bind(format!("-{LIVE_WINDOW_SECS} seconds"))
    .fetch_all(pool)
    .await
}
