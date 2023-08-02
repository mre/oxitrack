pub struct Base<'a> {
    pub title: &'a str,
    pub git_hash: &'static str,
}

impl<'a> Base<'a> {
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            git_hash: env!("GIT_HASH"),
        }
    }
}
