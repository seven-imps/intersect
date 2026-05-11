use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::router::{AppRoute, navigate_to};

#[component]
pub fn PageLink(
    route: AppRoute,
    #[prop(into)] text: TextProp,
    #[prop(into, optional)] title: Option<String>,
    #[prop(into, optional)] class: Option<String>,
) -> impl IntoView {
    let navigate = use_navigate();
    let target = route.nav_target();
    let title = title.unwrap_or_default();
    let class = format!("link {}", class.unwrap_or_default());
    let state = target.state;

    view! {
        <A href={target.url} {..} class=class title=title
            on:click=move |ev| {
                if state.is_some() {
                    ev.prevent_default();
                    navigate_to(&navigate, route.clone(), false);
                }
            }
        >
            {move || text.get()}
        </A>
    }
}
