use intersect_core::{
    ConnectionParams, Intersect, NetworkState, TypedReference, documents::AccountDocument,
};
use leptos::prelude::*;

use crate::{
    components::{Nav, base::PageLink, use_loading},
    router::AppRoute,
    util::{watch_to_new_signal, watch_to_signal},
};

/// returns the intersect instance from context.
/// panics if called outside of a Shell component.
pub fn use_intersect() -> Intersect {
    use_context::<Intersect>().expect("use_intersect called outside of Shell")
}

/// returns the reactive network state from context.
/// panics if called outside of a Shell component.
pub fn use_network_state() -> ReadSignal<NetworkState> {
    use_context::<ReadSignal<NetworkState>>().expect("use_network_state called outside of Shell")
}

/// returns the reactive account state from context.
/// None = anonymous session, Some = logged in with a persistent account.
/// panics if called outside of a Shell component.
pub fn use_account() -> ReadSignal<Option<TypedReference<AccountDocument>>> {
    use_context::<ReadSignal<Option<TypedReference<AccountDocument>>>>()
        .expect("use_account called outside of Shell")
}

#[component]
/// wrapper that sets up intersect initialisation, context, and basic page layout
pub fn Shell(children: ChildrenFn) -> impl IntoView {
    let init: RwSignal<bool> = RwSignal::new(false);
    let loading = use_loading();

    // save the reactive owner so we can provide context from inside the async task.
    let owner = Owner::current().expect("Intersect must run inside a reactive owner");
    let children = StoredValue::new(children);

    // pre-provide account signal so the header can use it before init completes
    let account: RwSignal<Option<TypedReference<AccountDocument>>> = RwSignal::new(None);
    provide_context::<ReadSignal<Option<TypedReference<AccountDocument>>>>(account.read_only());

    let init_task = move || async move {
        let node = Intersect::init(ConnectionParams::default()).await?;

        owner.with(|| {
            // set up intersect context
            provide_context::<Intersect>(node.clone());
            // bridge tokio watch channels to leptos signals and provide as context
            let network = watch_to_new_signal(node.network_watch());
            provide_context::<ReadSignal<NetworkState>>(network.read_only());
            watch_to_signal(account, node.account_watch());
        });

        init.set(true);
        Ok(())
    };

    // spin up intersect in the background
    leptos::task::spawn_local(async move {
        // result discarded and if this fails init will forever be false
        // this means no page content will render and nothing will work
        // but nothing could work anyway without intersect initialising so that's ok
        let _ = loading.run_fatal(init_task, "connecting...").await;
    });

    view! {
        <header id="header">
            <h1><PageLink route=AppRoute::Home text="./intersect/"/></h1>
            <Nav>
                <li><PageLink route=AppRoute::NewPost text="new post"/></li>
                <li><PageLink route=AppRoute::Account text={ move || if account.get().is_some() { "account" } else { "log in" } }/></li>
            </Nav>
        </header>

        <main id="main">
            <Show when=move || init.get()>
                {move || children.with_value(|c| c())}
            </Show>
        </main>

        <footer id="footer">
            <p>"<3"</p>
        </footer>
    }
}
