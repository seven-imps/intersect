use leptos::prelude::*;
use leptos_router::hooks::use_location;

#[component]
pub fn Nav(children: ChildrenFn) -> impl IntoView {
    let menu_checked = RwSignal::new(false);
    let children = StoredValue::new(children);

    let hash = use_location().hash;
    let state = use_location().state;
    // auto-close on navigation
    Effect::new(move |_| {
        hash.get();
        state.get();
        menu_checked.set(false);
    });

    view! {
        <nav id="hamburger-nav">
            <input title="menu" id="hamburger-input" type="checkbox"
                prop:checked=menu_checked
                on:change=move |ev| menu_checked.set(event_target_checked(&ev))
            />
            <label title="menu" id="hamburger" for="hamburger-input" tabindex=0>
                <span></span>
                <span></span>
                <span></span>
            </label>
            <ul>{ move || children.with_value(|c| c()) }</ul>
        </nav>
    }
}
