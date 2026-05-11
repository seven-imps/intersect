use intersect_core::{Document, MutableDocument, TypedReference};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::{use_intersect, util::watch_to_signal};

/// the standard async data shape for data from the network: None while loading, Some(Err) on failure, Some(Ok) when ready.
pub type NetworkSignal<T> = ReadSignal<Option<Result<T, String>>>;

/// one-shot document fetch
pub fn use_fetch<D: Document + 'static>(typed_ref: TypedReference<D>) -> NetworkSignal<D::View> {
    let intersect = use_intersect();
    let signal = RwSignal::new(None);
    spawn_local(async move {
        signal.set(Some(
            intersect.fetch(&typed_ref).await.map_err(|e| e.to_string()),
        ));
    });
    signal.read_only()
}

/// live document open
/// seeds the signal from the initial read then keeps it updated as new versions arrive
pub fn use_open<D: MutableDocument + 'static>(
    typed_ref: TypedReference<D>,
) -> NetworkSignal<D::View> {
    let intersect = use_intersect();
    let signal = RwSignal::new(None);
    spawn_local(async move {
        match intersect.open(&typed_ref).await {
            Err(e) => signal.set(Some(Err(e.to_string()))),
            Ok(doc) => {
                watch_to_signal(Some(signal), doc.updates, |v| {
                    Some(v.map_err(|e| e.to_string()))
                });
            }
        }
    });
    signal.read_only()
}

/// suspends child rendering behind a NetworkSignal,
/// showing a loading state or error message until the data is ready.
/// use `let:name` to bind the resolved value in the child view.
#[component]
pub fn NetworkSuspend<T, C, V>(signal: NetworkSignal<T>, children: C) -> impl IntoView
where
    T: Clone + Send + Sync + 'static,
    C: Fn(T) -> V + Send + Sync + 'static,
    V: IntoView + 'static,
{
    move || match signal.get() {
        None => view! { <p class="network-loading">"loading..."</p> }.into_any(),
        Some(Err(e)) => view! { <p class="network-error">"error: " {e}</p> }.into_any(),
        Some(Ok(value)) => children(value).into_any(),
    }
}
