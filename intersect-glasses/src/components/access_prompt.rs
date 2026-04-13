use std::str::FromStr;

use intersect_core::{Document, OpenedTrace, TypedReference, models::TraceSecret};
use leptos::prelude::*;

use crate::components::base::{Form, Modal, TextInput};

type UnlockResult<D> = Result<TypedReference<D>, String>;

/// encapsulates prompting the user for a secret / password
/// returns a signal tracking access resolution and a view containing the modal (if needed).
/// for already-unlocked traces the signal is seeded immediately and the view is empty.
pub fn use_access<D: Document + 'static>(
    trace: OpenedTrace<D>,
) -> (ReadSignal<Option<UnlockResult<D>>>, impl IntoView) {
    let reference: RwSignal<Option<Result<TypedReference<D>, String>>> = RwSignal::new(None);

    let modal = match trace {
        OpenedTrace::Unlocked(typed_ref) => {
            reference.set(Some(Ok(typed_ref)));
            ().into_any()
        }
        trace => {
            let on_resolve = Callback::new(move |result| reference.set(Some(result)));
            view! { <AccessPrompt trace on_resolve /> }.into_any()
        }
    };

    (reference.read_only(), modal)
}

#[component]
pub fn AccessPrompt<D: Document + 'static>(
    trace: OpenedTrace<D>,
    on_resolve: Callback<Result<TypedReference<D>, String>>,
) -> impl IntoView {
    let show = RwSignal::new(true);
    let input = RwSignal::new(String::new());
    let trace = StoredValue::new(trace);

    let validate = Callback::new(move |()| -> Result<TypedReference<D>, anyhow::Error> {
        let raw = input.get_untracked();
        trace.with_value(|o| match o {
            OpenedTrace::Unlocked(r) => {
                unreachable!("AccessPrompt should not be used with an unlocked trace")
            }
            OpenedTrace::Locked(locked) => {
                let secret = TraceSecret::from_str(&raw)?;
                Ok(locked.unlock(secret)?)
            }
            OpenedTrace::Protected(protected) => Ok(protected.unlock(&raw)?),
        })
    });

    let on_submit = Callback::new(move |typed_ref: TypedReference<D>| {
        show.set(false);
        on_resolve.run(Ok(typed_ref));
    });

    let input_label = trace.with_value(|o| match o {
        OpenedTrace::Unlocked(_) => {
            unreachable!("AccessPrompt should not be used with an unlocked trace")
        }
        OpenedTrace::Locked(_) => "secret key",
        OpenedTrace::Protected(_) => "password",
    });

    let dismiss = move || {
        show.set(false);
        on_resolve.run(Err(format!("no {} given", input_label)));
    };

    view! {
        <Modal show=show.read_only() title="unlock trace" on_close=Callback::new(move |()| dismiss())>
            <Form validate on_submit>
                <TextInput value=input id="access-input" label=input_label input_type="password" />
                <div class="access-prompt-actions">
                    <button type="button" on:click=move |_| dismiss()>"cancel"</button>
                    <button type="submit">"unlock"</button>
                </div>
            </Form>
        </Modal>
    }
}
