use intersect_core::{
    TypedReference,
    documents::{FragmentDocument, FragmentView},
};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::{components::Note, use_intersect};

#[component]
pub fn Fragment(typed_ref: TypedReference<FragmentDocument>) -> impl IntoView {
    let intersect = use_intersect();
    let result: RwSignal<Option<Result<FragmentView, String>>> = RwSignal::new(None);

    spawn_local(async move {
        result.set(Some(
            intersect.fetch(&typed_ref).await.map_err(|e| e.to_string()),
        ));
    });

    move || match result.get() {
        None => view! { <p>"loading..."</p> }.into_any(),
        Some(Err(e)) => view! { <p>"error: " {e}</p> }.into_any(),
        Some(Ok(fragment)) => {
            let mime = fragment.mime().as_ref().to_owned();
            if mime.starts_with("text/") {
                view! { <Note fragment /> }.into_any()
            } else {
                let len = fragment.data().len();
                view! {
                    <div class="fragment-unsupported">
                        <p>"unsupported format: " {mime}</p>
                        <p>{len} " bytes"</p>
                    </div>
                }
                .into_any()
            }
        }
    }
}
