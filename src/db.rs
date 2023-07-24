use std::ops::Deref;

use init_err::{InitErr, InitErrCtx};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions, PgPool,
};

use crate::config;

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
    pub async fn build(db_config: config::Database) -> Result<Self, InitErr> {
        let pool = {
            let options = PgConnectOptions::new_without_pgpass()
                .host(&db_config.host)
                .port(db_config.port)
                .username(&db_config.username)
                .password(&db_config.password)
                .database(&db_config.database)
                .disable_statement_logging();

            PgPoolOptions::new()
                .connect_with(options)
                .await
                .init_ctx("Failed to connect to the database!")?
        };

        sqlx::migrate!()
            .run(&pool)
            .await
            .init_ctx("Failed to run migrations!")?;

        Ok(Self { pool })
    }
}
