use leptos::prelude::*;
use leptos_router::components::A;

use crate::router::AppRoute;

#[component]
pub fn PageLink(
    route: AppRoute,
    #[prop(into)] text: TextProp,
    #[prop(into, optional)] title: Option<String>,
    #[prop(into, optional)] class: Option<String>,
) -> impl IntoView {
    let href = route.href(true);
    let title = title.unwrap_or_default();
    let class = format!("link {}", class.unwrap_or_default());

    view! {
        <A href={href} {..} class=class title=title>{move || text.get()}</A>
    }
}
