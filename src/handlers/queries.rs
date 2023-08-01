use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathQuery {
    path: String,
}

impl PathQuery {
    pub fn normalized(&self) -> &str {
        let path = self.path.trim_end_matches('/');

        if path.is_empty() {
            "/"
        } else {
            path
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PathQuery;

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
            let path = PathQuery {
                path: before.to_owned(),
            };

            assert_eq!(path.normalized(), after);
        }
    }
}
