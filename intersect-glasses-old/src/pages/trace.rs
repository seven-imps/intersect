use anyhow::Context;
use intersect_core::{models::Trace, IndexRecord};
use leptos::*;

use crate::{components::StatusContext, pages::navigate_to_view};

pub fn trace_link(trace: &Trace<IndexRecord>) -> String {
    format!("/#/trace/{}", trace)
}

#[component]
pub fn Trace(hash_args: String) -> impl IntoView {
    let status_context = expect_context::<StatusContext>();

    spawn_local(async move {
        let _ = status_context
            .run(
                || {
                    Trace::<IndexRecord>::from_str(&hash_args)
                        .with_context(|| "couldn't decode trace")
                },
                None,
            )
            .and_then(|trace| Ok(navigate_to_view(&trace)));
    });
}
