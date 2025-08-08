use intersect_core::log;
use leptos::*;
use leptos_router::use_location;

use crate::{
    components::{IfSome, PageLink, StatusContext},
    make_action,
    session::Session,
};

#[component]
pub fn Intersect(children: ChildrenFn) -> impl IntoView {
    let status_context = expect_context::<StatusContext>();

    let init_intersect = make_action!(move |_| {
        intersect_core::init().await;
        let _ = intersect_core::get_routing_context().await;
        status_context.clear();
    });

    status_context.set_status_view(move || {
        view! { <p class="initialising"> "connecting..." </p> }.into_view()
    });
    init_intersect.dispatch(());

    let children = store_value(children);

    // share our  login session as context
    log!("intersect component run");
    let session = create_rw_signal(Session::new());
    provide_context(session);

    let account_link = move || {
        if session.with(|s| s.blog_identity).is_none() {
            view! {<PageLink page="account" text="log in"/>}
        } else {
            view! {<PageLink page="account" text="account"/>}
        }
    };

    let menu_checked = create_rw_signal(false);
    // some reactivity magic to hide the menu whenever we navigate somewhere new
    let hash_memo = use_location().hash;
    let state_signal = use_location().state;
    create_effect(move |_| {
        hash_memo.get();
        state_signal.get();
        // hide menu
        menu_checked.set(false);
    });

    view! {
        <header id="header">
            <h1><PageLink page="home" text="./intersect/"/></h1>
            <nav id="hamburger-nav">
                <input title="menu" id="hamburger-input" type="checkbox"
                    // hook a reactive signal into the checkbox
                    prop:checked=menu_checked
                    on:change=move |ev| menu_checked.set(event_target_checked(&ev))
                />
                <label title="menu" id="hamburger" for="hamburger-input" tabindex=0>
                    // spans to style into the icon
                    <span></span>
                    <span></span>
                    <span></span>
                </label>
                <ul>
                    <li><PageLink page="post" text="new post"/></li>
                    <li> { account_link } </li>
                </ul>
            </nav>
        </header>

        <main id="main">
            <IfSome
                signal = init_intersect.value()
                view = move |_| children.with_value(|children| children())
            / >
        </main>

        <footer id="footer">
            <p>"<3"</p>
        </footer>
    }
}
