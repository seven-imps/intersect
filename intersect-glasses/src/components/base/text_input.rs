use leptos::ev::InputEvent;
use leptos::prelude::*;

#[component]
pub fn TextInput(
    value: RwSignal<String>,
    #[prop(into)] id: String,
    #[prop(into)] label: String,
    #[prop(into, default = "text".to_owned())] input_type: String,
    #[prop(into, default = "off".to_owned())] autocomplete: String,
    // when enabled, synthesises input events on the DOM node whenever value changes.
    // some password managers need this to pick up programmatic value updates.
    #[prop(optional)] reactive_events: bool,
    // optional reactive error message displayed below the input
    #[prop(optional)] error: Option<Signal<Option<String>>>,
) -> impl IntoView {
    let node_ref: NodeRef<leptos::html::Input> = NodeRef::new();

    let reactive_input_event = move || {
        if reactive_events {
            let _value = value.get();
            if let Some(node) = node_ref.get() {
                let event =
                    InputEvent::new("input").expect("input event to be created without error");
                node.dispatch_event(&event)
                    .expect("input event to fire without error");
            };
        }
    };

    view! {
        <div class="textinput">
            <label for=id.clone()>{label}</label>
            <input id=id type=input_type autocomplete=autocomplete node_ref=node_ref
                on:input=move |ev| value.set(event_target_value(&ev))
                prop:value=value
            />
            {reactive_input_event}
            <Show when=move || error.map(|e| e.get().is_some()).unwrap_or(false)>
                <p class="textinput-error">{move || error.and_then(|e| e.get())}</p>
            </Show>
        </div>
    }
}

#[component]
pub fn TextArea(
    value: RwSignal<String>,
    #[prop(into)] id: String,
    #[prop(into)] label: String,
) -> impl IntoView {
    view! {
        <div class="textarea">
            <label for=id.clone()>{label}</label>
            <textarea id=id
                prop:value=value
                on:input=move |ev| value.set(event_target_value(&ev))
            >
                {value.get_untracked()}
            </textarea>
        </div>
    }
}
