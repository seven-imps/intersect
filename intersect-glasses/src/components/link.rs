use ev::MouseEvent;
use intersect_core::{models::Trace, IndexRecord};
use leptos::*;
use leptos_router::A;

use crate::pages::{edit_link, trace_link, view_link};

#[component]
pub fn ActionLink(
    #[prop(into)] on_click: Callback<()>,
    #[prop(into)] text: String,
    #[prop(into, optional)] title: String,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let title = if title.is_empty() {
        text.clone()
    } else {
        title
    };

    let on_click_handler = move |ev: MouseEvent| {
        ev.prevent_default();
        on_click.call(());
    };

    let class = format!("link link-action {}", class);
    view! {
        <A class=class href="#" attr:title=title on:click=on_click_handler>{text}</A>
    }
}

#[component]
pub fn PageLink(
    #[prop(into)] page: String,
    #[prop(into)] text: String,
    #[prop(into, optional)] title: String,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    // let label = label.to_owned().clone();
    // let id = id.to_owned().clone();
    // let input_type = input_type.to_owned().clone();

    let title = if title.is_empty() {
        page.clone()
    } else {
        title
    };
    let href = format!("#/{}", page);
    let class = format!("link link-page {}", class);

    view! {
        <A class=class href=href attr:title=title>{text}</A>
    }
}

#[component]
pub fn IntersectLink(
    #[prop(into)] trace: Trace<IndexRecord>,
    #[prop(into)] text: String,
    #[prop(into, optional)] title: String,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let title = if title.is_empty() {
        trace.to_string()
    } else {
        title
    };
    let (href, state) = view_link(&trace);
    let class = format!("link link-intersect {}", class);

    view! {
        <A class=class href=href state=state.into() attr:title=title>{text}</A>
    }
}

#[component]
pub fn IntersectTraceLink(
    #[prop(into)] trace: Trace<IndexRecord>,
    #[prop(into)] text: String,
    #[prop(into, optional)] title: String,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let title = if title.is_empty() {
        trace.to_string()
    } else {
        title
    };
    let href = trace_link(&trace);
    let class = format!("link link-intersect-trace {}", class);

    view! {
        <A class=class href=href attr:title=title>{text}</A>
    }
}

#[component]
pub fn IntersectEditLink(
    #[prop(into)] trace: Trace<IndexRecord>,
    #[prop(into)] text: String,
    #[prop(into, optional)] title: String,
    #[prop(into, optional)] class: String,
) -> impl IntoView {
    let title = if title.is_empty() {
        trace.to_string()
    } else {
        title
    };
    let (href, state) = edit_link(&trace);
    let class = format!("link link-intersect-edit {}", class);

    view! {
        <A class=class href=href state=state.into() attr:title=title>{text}</A>
    }
}
