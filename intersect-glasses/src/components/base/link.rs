use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn PageLink(
    #[prop(into)] page: String,
    #[prop(into)] text: TextProp,
    #[prop(into, optional)] title: Option<String>,
    #[prop(into, optional)] class: Option<String>,
) -> impl IntoView {
    let title = title.unwrap_or_else(|| page.clone());
    let href = format!("#/{}", page);
    let class = format!("link {}", class.unwrap_or_default());

    view! {
        <A href={href} {..} class=class title=title>{move || text.get()}</A>
    }
}
