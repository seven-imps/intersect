use leptos::portal::Portal;
use leptos::prelude::*;

use super::Card;

#[component]
pub fn Modal(
    children: ChildrenFn,
    #[prop(into)] show: Signal<bool>,
    #[prop(into)] title: TextProp,
    // called when the modal is closed via the X button or escape key
    // (not called when closed by setting show to false!)
    #[prop(optional)] on_close: Option<Callback<()>>,
    #[prop(optional)] closedby: Option<String>,
) -> impl IntoView {
    let dialog_ref: NodeRef<leptos::html::Dialog> = NodeRef::new();
    let children = StoredValue::new(children);
    let title = StoredValue::new(title);
    let closedby = StoredValue::new(closedby.unwrap_or("none".to_string()));

    let show_modal = move || {
        if let Some(dialog) = dialog_ref.get_untracked() {
            dialog
                .show_modal()
                .expect("modal to display without errors")
        }
    };

    let close_modal = move || {
        if let Some(dialog) = dialog_ref.get_untracked() {
            dialog.close()
        }
    };

    // this is a bit janky since the show signal isn't updated
    // when you close the modal by e.g. pressing escape
    // but since this check will only ever re-run when the signal is updated
    // it'll never actually matter that it's out of sync with the true modal state
    Effect::new(move |_| {
        // the animation frame ensures the modal element is in the DOM before calling show_modal
        match show.get() {
            true => request_animation_frame(show_modal),
            false => close_modal(),
        }
    });

    // combine close_modal with the user callback so Card's on_close does both
    let on_close = Callback::new(move |()| {
        close_modal();
        if let Some(cb) = on_close {
            cb.run(());
        }
    });

    view! {
        <Portal>
            <dialog class="modal-dialog" node_ref=dialog_ref
                // cancel fires on Escape before the dialog closes natively
                on:cancel=move |_: leptos::ev::Event| { on_close.run(()); }
                closedby=move || closedby.with_value(|s| s.clone())
            >
                <Card title=move || title.with_value(|t| t.get()) on_close=Some(on_close)>
                    {children.with_value(|children| children())}
                </Card>
            </dialog>
        </Portal>
    }
}
