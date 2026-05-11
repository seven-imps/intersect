use leptos::prelude::*;
use tokio::sync::watch;

/// bridges a tokio watch receiver to a leptos signal, keeping it updated as the receiver changes.
/// seeds from the receiver's current value
/// pass `Some(signal)` to write into an existing signal or `None` to create a fresh one.
pub fn watch_to_signal<T, U>(
    signal: Option<RwSignal<U>>,
    mut rx: watch::Receiver<T>,
    map: impl Fn(T) -> U + Send + 'static,
) -> RwSignal<U>
where
    T: Clone + Send + Sync + 'static,
    U: Send + Sync + 'static,
{
    let initial = map(rx.borrow_and_update().clone());
    let signal = match signal {
        Some(s) => {
            s.set(initial);
            s
        }
        None => RwSignal::new(initial),
    };
    leptos::task::spawn_local(async move {
        while rx.changed().await.is_ok() {
            signal.set(map(rx.borrow_and_update().clone()));
        }
    });
    signal
}
