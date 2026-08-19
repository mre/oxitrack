use crate::states::InnerAppState;

pub struct Base {
    pub title: String,
    pub utc_offset: &'static str,
}

impl Base {
    pub fn new(state: &'static InnerAppState, title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            utc_offset: state.utc_offset_str,
        }
    }
}
