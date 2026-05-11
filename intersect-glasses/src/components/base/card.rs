use leptos::prelude::*;
use phosphor_leptos::{Icon, IconWeight, X};

#[derive(Default, PartialEq, Clone, Copy)]
pub enum CardVariant {
    #[default]
    Default,
    Error,
}

#[component]
pub fn Card(
    children: Children,
    #[prop(optional, into)] title: Option<TextProp>,
    #[prop(optional, into)] on_close: Signal<Option<Callback<()>>>,
    #[prop(optional)] variant: CardVariant,
) -> impl IntoView {
    let title = title.map(StoredValue::new);
    let show_header = move || title.is_some() || on_close.get().is_some();

    view! {
        <div class="card-body" class:card-error=move || variant == CardVariant::Error>
            <Show when=show_header>
                <div class="card-header">
                    {title.map(|t| view! {
                        <h1 class="card-title">{move || t.with_value(|t| t.get())}</h1>
                    })}
                    {move || on_close.get().map(|cb| view! {
                        <button title="close" class="card-close button-icon"
                            on:click=move |_| cb.run(())>
                            <Icon icon=X weight=IconWeight::Bold />
                        </button>
                    })}
                </div>
            </Show>
            <div class="card-content">
                {children()}
            </div>
        </div>
    }
}
