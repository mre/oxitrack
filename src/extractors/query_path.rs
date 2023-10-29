use axum::{extract::FromRequestParts, http::request::Parts};
use axum_ctx::{RespErr, RespErrCtx, RespErrExt, Status};
use serde::Deserialize;
use sqlx::PgPool;

use crate::states::InnerAppState;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
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

        if path.is_empty() {
            "/"
        } else {
            path
        }
    }

    pub async fn normalized_with_id(&self, pool: &PgPool) -> Result<PathId, RespErr> {
        let normalized = self.normalized();

        let id = sqlx::query!(
            "SELECT id FROM paths
            WHERE path = $1",
            normalized,
        )
        .fetch_one(pool)
        .await
        .ctx(Status::NotFound)
        .user_msg(|| format!("Path {normalized} not found!"))?
        .id;

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

pub struct OptionalPathId(pub Option<i64>);

#[async_trait::async_trait]
impl FromRequestParts<&'static InnerAppState> for OptionalPathId {
    type Rejection = RespErr;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &&'static InnerAppState,
    ) -> Result<Self, Self::Rejection> {
        let query = match parts.uri.query() {
            Some(v) => v,
            None => return Ok(Self(None)),
        };

        let path = serde_urlencoded::from_str::<QueryPath>(query)
            .ctx(Status::BadRequest)
            .user_msg("Failed to deserialize the `path` query parameter!")?;

        let path_id = path.normalized_with_id(&state.pool).await?.path_id;

        Ok(Self(Some(path_id)))
    }
}
