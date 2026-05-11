use leptos::*;

#[component]
pub fn Selectable(
    #[prop(into)] text: String,
    #[prop(into, optional)] label: String,
) -> impl IntoView {
    let label = format!("{label}: ");
    view! {
        <div class="selectable">
            <label for="selectable-text" class="selectable-label">{label}</label>
            <p id="selectable-text" class="selectable-text" tabindex="0">{text}</p>
        </div>
    }
}
