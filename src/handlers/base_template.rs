pub struct Base<'a> {
    pub title: &'a str,
}

impl<'a> Base<'a> {
    pub const fn new(title: &'a str) -> Self {
        Self { title }
    }
}
