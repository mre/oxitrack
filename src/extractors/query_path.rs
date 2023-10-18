use oxi_axum_helpers::{RespErr, RespErrCtx, RespErrExt, Status};
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryPath {
    path: String,
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

    pub async fn normalized_with_id(&self, pool: &PgPool) -> Result<(&str, i64), RespErr> {
        let normalized = self.normalized();

        let id = sqlx::query!(
            "SELECT id FROM paths
            WHERE path = $1",
            normalized,
        )
        .fetch_one(pool)
        .await
        .ctx(Status::NotFound)
        .err_msg(|| format!("Path {normalized} not found!"))?
        .id;

        Ok((normalized, id))
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
