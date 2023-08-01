use std::ops::Deref;

use oxi_axum_helpers::{DBConfig, InitErr, InitErrCtx, RespErr, RespErrCtx, RespErrExt, Status};
use serde::Serialize;
use sqlx::PgPool;
use time::OffsetDateTime;

pub struct Database {
    pool: PgPool,
}

impl Deref for Database {
    type Target = PgPool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

impl Database {
    pub async fn build(db_config: DBConfig) -> Result<Self, InitErr> {
        let pool = db_config.try_into_pool().await?;

        sqlx::migrate!()
            .run(&pool)
            .await
            .init_ctx("Failed to run migrations!")?;

        Ok(Self { pool })
    }
}

pub struct Id {
    pub id: i64,
}

pub struct TimeStamp {
    pub timestamp: OffsetDateTime,
}

#[derive(Serialize)]
pub struct Count {
    pub path: String,
    pub count: i64,
}

impl Count {
    pub async fn query_all(pool: &PgPool) -> Result<Vec<Self>, RespErr> {
        sqlx::query_as!(
            Self,
            r#"SELECT path, COUNT(*) AS "count!" FROM paths
            INNER JOIN visits ON visits.path_id = paths.id
            GROUP BY path
            ORDER BY path"#
        )
        .fetch_all(pool)
        .await
        .ctx(Status::Internal)
        .err_msg("Counts query failed!")
    }
}
