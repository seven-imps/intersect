use intersect_core::models::{Trace, TraceError};
use leptos::*;

use crate::{components::TextInput, pages::trace_link};

#[component]
pub fn Lookup() -> impl IntoView {
    let trace_field = create_rw_signal("".to_string());

    let decoded_trace = move || Trace::from_str(&trace_field.get());

    let input_view = move |e: TraceError| {
        view! {
            <TextInput value=trace_field id="trace" label="trace: " />
            <Show
                when = move || !trace_field.get().is_empty()
            >
                <div class="trace">
                    <p>"error:  " {e.to_string()}</p>
                </div>
            </Show>
        }
        .into_view()
    };

    move || match decoded_trace() {
        Ok(t) => {
            let navigate = leptos_router::use_navigate();
            navigate(&trace_link(&t), Default::default());
            view! {}.into_view()
        }
        Err(e) => input_view(e),
    }
}
