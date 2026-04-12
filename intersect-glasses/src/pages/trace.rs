use leptos::prelude::*;

/// stub — will fetch and render the trace once core integration is ready
#[component]
pub fn TracePage(args: String) -> impl IntoView {
    view! {
        <p>"trace: " {args}</p>
    }
}
