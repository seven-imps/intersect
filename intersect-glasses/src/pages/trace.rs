use std::str::FromStr;

use intersect_core::{
    TypedTrace,
    documents::FragmentDocument,
    models::{DocumentType, Trace},
};
use leptos::prelude::*;

use crate::components::{FragmentDisplay, use_access};

#[component]
pub fn TracePage(trace: String) -> impl IntoView {
    let trace = match Trace::from_str(&trace) {
        Ok(trace) => trace,
        Err(e) => return view! { <p>"invalid trace: " {e.to_string()}</p> }.into_any(),
    };

    // TODO: this match feels messy with the unreachable Err case...
    match *trace.document_type() {
        DocumentType::Fragment => match trace.into_typed::<FragmentDocument>() {
            Ok(opened) => fragment_page(opened).into_any(),
            Err(_) => unreachable!("unexpected document type"),
        },
        other => view! { <p>"unsupported document type: " {format!("{other:?}")}</p> }.into_any(),
    }
}

fn fragment_page(opened: TypedTrace<FragmentDocument>) -> impl IntoView {
    let (resolved, access_view) = use_access(opened);

    view! {
        {access_view}
        {move || match resolved.get() {
            None => ().into_any(),
            Some(Ok(fragment_ref)) => view! { <FragmentDisplay fragment_ref /> }.into_any(),
            Some(Err(e)) => view! { <p>"error: " {e}</p> }.into_any(),
        }}
    }
}
