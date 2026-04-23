use std::str::FromStr;

use intersect_core::models::Trace;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::{
    components::base::TextInput,
    router::{AppRoute, navigate_to},
};

#[component]
pub fn Lookup() -> impl IntoView {
    let trace_field = RwSignal::new(String::new());
    let navigate = use_navigate();

    let decoded_trace = Memo::new(move |_| Trace::from_str(&trace_field.get()));
    let error = Memo::new(
        move |_| match (trace_field.get().is_empty(), decoded_trace.get()) {
            (false, Err(e)) => Some(e.to_string()),
            _ => None,
        },
    );

    // navigate to the trace page as soon as the input decodes successfully
    Effect::new(move |_| {
        if let Ok(trace) = decoded_trace.get() {
            navigate_to(&navigate, AppRoute::Trace(trace.to_string()));
        }
    });

    view! {
        <TextInput value=trace_field id="trace" label="trace: " error=Signal::from(error) />
    }
}
