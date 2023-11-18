use crate::states::InnerAppState;

pub struct Base<'a> {
    pub title: &'a str,
    pub utc_offset: &'static str,
}

impl<'a> Base<'a> {
    pub const fn new(state: &'static InnerAppState, title: &'a str) -> Self {
        Self {
            title,
            utc_offset: state.utc_offset_str,
        }
    }
}
