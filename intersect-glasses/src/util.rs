use leptos::prelude::*;
use tokio::sync::watch;

/// bridges a tokio watch receiver to a leptos signal.
pub fn watch_to_signal<T>(mut rx: watch::Receiver<T>) -> RwSignal<T>
where
    T: Clone + Send + Sync + 'static,
{
    // seed with initial value
    let signal = RwSignal::new(rx.borrow_and_update().clone());
    // and spawn a task to check for updates
    leptos::task::spawn_local(async move {
        while rx.changed().await.is_ok() {
            signal.set(rx.borrow_and_update().clone());
        }
    });
    signal
}
