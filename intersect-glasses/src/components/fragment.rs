use intersect_core::{TypedReference, documents::FragmentDocument};
use leptos::prelude::*;

use crate::components::{NetworkSuspend, Note, use_fetch};

#[component]
pub fn FragmentDisplay(fragment_ref: TypedReference<FragmentDocument>) -> impl IntoView {
    let signal = use_fetch(fragment_ref);

    view! {
        <NetworkSuspend signal let:fragment>
            {
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
        </NetworkSuspend>
    }
}
