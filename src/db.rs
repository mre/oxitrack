use std::ops::Deref;

use oxi_axum_helpers::{DBConfig, InitErr, InitErrCtx};
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
