use intersect_core::{models::Trace, IndexRecord};
use leptos::*;

use crate::components::{IntersectTraceLink, Selectable};

#[component]
pub fn ShareTrace(#[prop(into)] trace: Trace<IndexRecord>) -> impl IntoView {
    view! {
        <div class="share-trace">
            <Selectable text=trace.to_string() label="trace"/>
            <IntersectTraceLink trace=trace text="trace link"/>
        </div>
    }
}
