use intersect_core::models::UnlockedTrace;
use intersect_core::{Identity, IndexRecord};
use leptos::*;

use crate::components::{Collapsible, EditorNew, LogoutButton};
use crate::session::Session;

#[component]
pub fn Post() -> impl IntoView {
    let trace_signal: RwSignal<Option<UnlockedTrace<IndexRecord>>> = create_rw_signal(None);

    let session = expect_context::<RwSignal<Session>>();
    let identity = create_memo(move |_| {
        session
            // use our session login if we're logged in
            .with(|s| s.blog_identity)
            // else use a throwaway identity if not logged in
            .unwrap_or_else(|| Identity::random())
    });

    let posting_as = move || {
        session.with(|s| s.blog_identity).map_or_else(
            || "anonymous".to_owned(),
            |identity| identity.shard().to_string(),
        )
    };

    let collapse_view = move || {
        if session.with(|s| s.is_logged_in()) {
            view! { <LogoutButton /> }.into_view()
        } else {
            view! {
                <p> "a single use keypair will be generated for this post." </p>
                <p> "you will never be able to edit this post!" </p>
            }
            .into_view()
        }
    };

    view! {
        <Collapsible summary = move || view! {"posting as " {posting_as} }>
            // <Login />
            {collapse_view}
        </Collapsible>

        <EditorNew identity=identity trace_out=trace_signal.write_only() initial_text="<3"/>

    }
}
