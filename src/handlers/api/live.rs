use axum::{extract::State, response::Html};

use crate::states::AppState;

pub async fn get(State(state): AppState) -> Html<String> {
    let count = state.visitor_states.live_count();

    let html = if count == 0 {
        String::new()
    } else {
        format!(
            r#"<span class="live-indicator" title="{count} visitor{s} on site right now">
  <span class="live-dot"></span>{count}
</span>"#,
            s = if count == 1 { "" } else { "s" },
        )
    };

    Html(html)
}
