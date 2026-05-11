use intersect_core::{
    models::{Trace, UnlockedTrace},
    IndexRecord,
};
use leptos::*;

use crate::{components::EditorExisting, session::Session};

/// returns a (url, state) pair
pub fn edit_link(trace: &Trace<IndexRecord>) -> (String, String) {
    ("/#/edit".to_owned(), trace.to_string())
}

#[component]
pub fn Edit(state: String) -> impl IntoView {
    let trace = Trace::<IndexRecord>::from_str(&state).expect("invalid link");

    let session = expect_context::<RwSignal<Session>>();
    let identity = session.with_untracked(|s| s.blog_identity);

    let unauthorised_view = move || view! { <p> "unauthorised" </p> }.into_view();

    // unpack identity and ensure we're logged in
    let Some(identity) = identity else {
        return unauthorised_view();
    };

    // // ... and our identity matches the trace
    // if identity.shard() != link.reference().shard() {
    //     return unauthorised_view();
    // }

    // // prompt for secret if the link is locked
    // let Link::Unlocked(unlocked_link) = link else {
    //     // TODO: add password prompt
    //     return unauthorised_view();
    // };

    let Ok(unlocked_trace) = UnlockedTrace::<IndexRecord>::try_from(trace) else {
        return unauthorised_view();
    };

    view! {
        <EditorExisting identity=identity trace=unlocked_trace />
    }
    .into_view()
}
