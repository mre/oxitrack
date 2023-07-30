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
