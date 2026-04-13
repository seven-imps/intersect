use std::str::FromStr;

use intersect_core::{
    OpenedTrace,
    documents::FragmentDocument,
    models::{DocumentType, Trace},
};
use leptos::prelude::*;

use crate::components::{Fragment, use_access};

#[component]
pub fn TracePage(args: String) -> impl IntoView {
    let trace = match Trace::from_str(&args) {
        Ok(trace) => trace,
        Err(e) => return view! { <p>"invalid trace: " {e.to_string()}</p> }.into_any(),
    };

    // TODO: this match feels messy with the unreachable Err case...
    match *trace.document_type() {
        DocumentType::Fragment => match trace.open::<FragmentDocument>() {
            Ok(opened) => fragment_page(opened).into_any(),
            Err(_) => unreachable!("unexpected document type"),
        },
        other => view! { <p>"unsupported document type: " {format!("{other:?}")}</p> }.into_any(),
    }
}

fn fragment_page(opened: OpenedTrace<FragmentDocument>) -> impl IntoView {
    let (resolved, access_view) = use_access(opened);

    view! {
        {access_view}
        {move || match resolved.get() {
            None => ().into_any(),
            Some(Ok(typed_ref)) => view! { <Fragment typed_ref /> }.into_any(),
            Some(Err(e)) => view! { <p>"error: " {e}</p> }.into_any(),
        }}
    }
}
