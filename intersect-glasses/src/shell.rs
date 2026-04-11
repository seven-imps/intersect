use intersect_core::{ConnectionParams, Intersect, IntersectError, NetworkState, log};
use leptos::prelude::*;

use crate::util::watch_to_signal;

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

#[component]
/// wrapper that sets up intersect initialisation, context, and basic page layout
pub fn Shell(children: ChildrenFn) -> impl IntoView {
    // None = initialising, Some(Err) = failed, Some(Ok) = ready
    let init: RwSignal<Option<Result<(), IntersectError>>> = RwSignal::new(None);

    // save the reactive owner so we can provide context from inside the async task.
    let owner = Owner::current().expect("Intersect must run inside a reactive owner");
    let children = StoredValue::new(children);

    leptos::task::spawn_local(async move {
        match Intersect::init(ConnectionParams::default()).await {
            Ok(node) => {
                // yay!
                owner.with(|| {
                    // intersect context
                    provide_context::<Intersect>(node.clone());
                    // and then also set up a signal that tracks the network state
                    let network = watch_to_signal(node.network_watch());
                    provide_context::<ReadSignal<NetworkState>>(network.read_only());
                });
                init.set(Some(Ok(())));
            }
            Err(e) => {
                log!("intersect init failed: {e}");
                init.set(Some(Err(e)));
            }
        }
    });

    move || match init.get() {
        None => view! { <p class="connecting">"connecting..."</p> }.into_any(),
        Some(Err(e)) => {
            view! { <p class="error">"failed to connect: " {e.to_string()}</p> }.into_any()
        }
        Some(Ok(())) => children.with_value(|c| c()).into_any(),
    }
}
