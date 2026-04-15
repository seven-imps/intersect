use leptos::prelude::*;
use tokio::sync::watch;

/// bridges a tokio watch receiver to a new leptos signal, seeded from the receiver's current value.
pub fn watch_to_new_signal<T>(mut rx: watch::Receiver<T>) -> RwSignal<T>
where
    T: Clone + Send + Sync + 'static,
{
    let signal = RwSignal::new(rx.borrow_and_update().clone());
    leptos::task::spawn_local(async move {
        while rx.changed().await.is_ok() {
            signal.set(rx.borrow_and_update().clone());
        }
    });
    signal
}

/// bridges a tokio watch receiver into an existing leptos signal.
/// use this when the signal needs to be provided as context before the receiver is available.
/// seeds the signal from the receiver's current value on entry.
pub fn watch_to_signal<T>(signal: RwSignal<T>, mut rx: watch::Receiver<T>)
where
    T: Clone + Send + Sync + 'static,
{
    // seed immediately so the signal reflects the current watch value
    signal.set(rx.borrow_and_update().clone());
    leptos::task::spawn_local(async move {
        while rx.changed().await.is_ok() {
            signal.set(rx.borrow_and_update().clone());
        }
    });
}
