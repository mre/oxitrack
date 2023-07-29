use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathQuery {
    path: String,
}

impl PathQuery {
    pub fn trimmed(&self) -> &str {
        self.path.trim_end_matches('/')
    }
}
