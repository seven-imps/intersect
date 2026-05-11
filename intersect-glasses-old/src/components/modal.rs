use leptos::*;
use phosphor_leptos::{Icon, IconWeight};

#[component]
pub fn Modal(
    children: ChildrenFn,
    #[prop(into)] show: Signal<bool>,
    #[prop(into)] title: String,
) -> impl IntoView {
    let dialog_ref: NodeRef<html::Dialog> = create_node_ref();
    let children = store_value(children);

    let show_modal = move || {
        if let Some(dialog) = dialog_ref.get_untracked() {
            dialog.show_modal().expect("modal to dispay without errors")
        }
    };

    let close_modal = move || {
        if let Some(dialog) = dialog_ref.get_untracked() {
            dialog.close()
        }
    };

    create_effect(move |_| {
        // this is a bit janky since the show signal isn't updated
        // when you close the modal be e.g pressing escape
        // but since this check will only ever re-run when the signal is updated
        // it'll never actually matter that it's out of sync with the true modal state
        match show.get() {
            // the animation frame is to ensure that the modal element is in the DOM
            true => request_animation_frame(move || show_modal()),
            false => close_modal(),
        }
    });

    view! {
        <Portal>
            <dialog class="modal-dialog" node_ref=dialog_ref >
                <div class="modal-body">
                    <div class="modal-header">
                        <h1 class="modal-title" autofocus=true> { &title } </h1>
                        <button title="close" class="modal-close-x button-icon" on:click=move |_| close_modal()>
                            <Icon icon=phosphor_leptos::X weight=IconWeight::Bold />
                        </button>
                    </div>
                    <div class="modal-content">
                        {children.with_value(|children| children())}
                    </div>
                    // <button class="modal-close" type="button" on:click=move |_| close_modal()>"close"</button>
                </div>
            </dialog>
            // {check_show}
        </Portal>
    }
}
