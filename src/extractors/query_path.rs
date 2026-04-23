use axum_ctx::{RespErrCtx, RespErrExt, RespResult, StatusCode};
use serde::Deserialize;
use sqlx::Row;

use crate::db::DbPool;

#[derive(Deserialize)]
pub struct QueryPath {
    path: String,
}

pub struct PathId<'a> {
    pub path: &'a str,
    pub path_id: i64,
}

impl QueryPath {
    pub fn normalized(&self) -> &str {
        let path = self.path.trim_end_matches('/');

        if path.is_empty() { "/" } else { path }
    }

    pub async fn normalized_with_id(&self, pool: &DbPool) -> RespResult<PathId<'_>> {
        let normalized = self.normalized();

        let row = sqlx::query("SELECT id FROM paths WHERE path = ? LIMIT 1")
            .bind(normalized)
            .fetch_one(pool)
            .await
            .ctx(StatusCode::NOT_FOUND)
            .user_msg(|| format!("Path {normalized} not found!"))?;

        let id: i64 = row.get("id");

        Ok(PathId {
            path: normalized,
            path_id: id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::QueryPath;

    #[test]
    fn path_normalization() {
        for (before, after) in [
            ("/blog", "/blog"),
            ("", "/"),
            ("/blog/", "/blog"),
            ("/", "/"),
            ("//", "/"),
            ("/nested/path", "/nested/path"),
            ("/nested/path///", "/nested/path"),
        ] {
            let path = QueryPath {
                path: before.to_owned(),
            };

            assert_eq!(path.normalized(), after);
        }
    }
}
