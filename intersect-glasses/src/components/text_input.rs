use ev::InputEvent;
use leptos::*;

#[component]
pub fn TextInput(
    value: RwSignal<String>,
    #[prop(into)] id: String,
    #[prop(into)] label: String,
    #[prop(into, default = "text".to_owned())] input_type: String,
    #[prop(into, default = "off".to_owned())] autocomplete: String,
    #[prop(into, default = false)] reactive_events: bool,
) -> impl IntoView {
    let node_ref: NodeRef<html::Input> = create_node_ref();

    // some password managers don't seem to pick up form field values correctly
    // when the value prop is changed without an input (or keydown) event
    // to fix that we can make a reactive funtion here which sends
    // events to the correct dom node whenever the value changes
    let reactive_input_event = move || {
        if reactive_events {
            // don't need it, but calling get so we get reactivity
            let _value = value.get();
            // make sure everything is rendred
            if let Some(node) = node_ref.get() {
                // and then create and send an event!
                // (see: https://developer.mozilla.org/en-US/docs/Web/Events/Creating_and_triggering_events)
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
                prop:value=move || value.get()
                on:input=move |ev| value.set(event_target_value(&ev))
            >
                { value.get_untracked() } // plain-text initial value, does not change if the signal changes
            </textarea>
        </div>
    }
}
