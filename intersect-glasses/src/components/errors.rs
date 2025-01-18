use leptos::*;

#[component]
pub fn Errors(errors: RwSignal<Errors>) -> impl IntoView {
    view! {
        <h1>"something broke"</h1>
        <ul>
            {move || {
                errors
                    .get()
                    .into_iter()
                    .map(|(_, e)| view! { <li>{e.to_string()}</li> })
                    .collect_view()
            }}

        </ul>
    }
}

pub fn error_view(errors: RwSignal<Errors>) -> leptos::View {
    view! { <Errors errors=errors/> }.into_view()
}

pub fn empty_view() -> View {
    view! {}.into_view()
}
