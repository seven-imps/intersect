use intersect_core::{
    models::{Access, Trace, TraceError, UnlockedTrace},
    IndexRecord,
};
use leptos::*;

use crate::components::Document;

/// returns a (url, state) pair
pub fn view_link(link: &Trace<IndexRecord>) -> (String, String) {
    ("/#/view".to_owned(), link.to_string())
}

pub fn navigate_to_view(link: &Trace<IndexRecord>) {
    let (url, state) = view_link(&link);
    leptos_router::use_navigate()(
        &url,
        leptos_router::NavigateOptions {
            state: leptos_router::State(Some(state.into())),
            // to make sure we don't get in redirect loops
            // when navigating backwards through history
            replace: true,
            ..Default::default()
        },
    );
}

#[component]
// pub fn View(state: String) -> impl IntoView {
pub fn View(state: String) -> impl IntoView {
    let decoded_trace = Trace::<IndexRecord>::from_str(&state);

    let error_view = move |e: TraceError| {
        view! {
            <div class="view">
                <p>"error:  " {move || e.to_string()}</p>
            </div>
        }
        .into_view()
    };

    let trace = match decoded_trace {
        Err(error) => return error_view(error),
        Ok(trace) => trace,
    };

    let locked_view = move || {
        // TODO: add page here to let user input a key
        view! { <p>"[locked]"</p> }.into_view()
    };

    let protected_view = move |_protected_secret| {
        // TODO: add page here to let user input a password
        view! { <p>"[password protected]"</p> }.into_view()
    };

    match trace.access() {
        Access::Locked => locked_view(),
        Access::Protected(protected_secret) => protected_view(protected_secret),
        Access::Unlocked(secret) => view! {
            <Document trace=UnlockedTrace::new(trace.key().clone(), secret.clone()) />
        },
    }
}
