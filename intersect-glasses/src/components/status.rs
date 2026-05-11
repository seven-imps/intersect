use std::{
    future::Future,
    sync::atomic::{AtomicU64, Ordering},
};

use anyhow::Error;
use leptos::{context::Provider, portal::Portal, prelude::*};

use super::base::{Card, CardVariant};

#[derive(Clone)]
struct LoadingEntry {
    id: u64,
    message: String,
}

#[derive(Clone)]
struct ErrorEntry {
    id: u64,
    error: String,
    dismissable: bool,
}

/// app-wide loading and error overlay handler.
/// use `use_loading()` hook to access (must be inside a `StatusProvider`)
#[derive(Clone, Copy)]
pub struct LoadingHandler {
    loading: RwSignal<Vec<LoadingEntry>>,
    errors: RwSignal<Vec<ErrorEntry>>,
    next_id: StoredValue<AtomicU64>,
}

impl LoadingHandler {
    fn new() -> Self {
        LoadingHandler {
            loading: RwSignal::new(vec![]),
            errors: RwSignal::new(vec![]),
            next_id: StoredValue::new(AtomicU64::new(0)),
        }
    }

    fn alloc_id(&self) -> u64 {
        self.next_id
            .with_value(|counter| counter.fetch_add(1, Ordering::Relaxed))
    }

    fn push_loading(&self, message: &str) -> u64 {
        let id = self.alloc_id();
        self.loading.update(|v| {
            v.push(LoadingEntry {
                id,
                message: message.to_owned(),
            })
        });
        id
    }

    fn pop_loading(&self, id: u64) {
        self.loading.update(|v| v.retain(|entry| entry.id != id));
    }

    fn push_error(&self, error: &Error, dismissable: bool) {
        let id = self.alloc_id();
        self.errors.update(|v| {
            v.push(ErrorEntry {
                id,
                error: error.to_string(),
                dismissable,
            })
        });
    }

    /// push a dismissable error entry.
    pub fn error(&self, error: &Error) {
        self.push_error(error, true);
    }

    /// push a non-dismissable error entry (for unrecoverable failures)
    pub fn fatal_error(&self, error: &Error) {
        self.push_error(error, false);
    }

    fn dismiss_errors(&self) {
        self.errors.update(|v| v.retain(|e| !e.dismissable));
    }

    /// run an async operation behind a loading indicator.
    /// any error is surfaced as a dismissable error overlay.
    pub async fn run<F, O, Fu>(&self, function: F, message: &str) -> Result<O, Error>
    where
        F: FnOnce() -> Fu,
        Fu: Future<Output = Result<O, Error>>,
    {
        let id = self.push_loading(message);
        let result = function().await;
        self.pop_loading(id);
        if let Err(ref error) = result {
            self.error(error);
        }
        result
    }

    /// run an async operation behind a loading indicator.
    /// any error is surfaced as a non-dismissable error overlay (for unrecoverable failures)
    pub async fn run_fatal<F, O, Fu>(&self, function: F, message: &str) -> Result<O, Error>
    where
        F: FnOnce() -> Fu,
        Fu: Future<Output = Result<O, Error>>,
    {
        let id = self.push_loading(message);
        let result = function().await;
        self.pop_loading(id);
        if let Err(ref error) = result {
            self.fatal_error(error);
        }
        result
    }

    /// run an async operation with a loading indicator, without surfacing errors.
    /// use this when errors are handled elsewhere or don't need to be shown to the user.
    pub async fn run_silent<F, O, Fu>(&self, function: F, message: &str) -> O
    where
        F: FnOnce() -> Fu,
        Fu: Future<Output = O>,
    {
        let id = self.push_loading(message);
        let result = function().await;
        self.pop_loading(id);
        result
    }
}

/// returns the loading handler from context.
/// panics if called outside of a `StatusProvider`.
pub fn use_loading() -> LoadingHandler {
    use_context::<LoadingHandler>().expect("use_loading called outside of StatusProvider")
}

#[component]
pub fn StatusProvider(children: ChildrenFn) -> impl IntoView {
    let handler = LoadingHandler::new();
    let children = StoredValue::new(children);

    let has_loading = move || !handler.loading.get().is_empty();
    let has_errors = move || !handler.errors.get().is_empty();
    let is_active = move || has_loading() || has_errors();
    let all_dismissable = move || handler.errors.get().iter().all(|e| e.dismissable);

    view! {
        <Provider value=handler>
            <Portal>
                <div class="status-overlay" aria-expanded=move || is_active().to_string()>
                    <div class="status-backdrop"></div>
                    <div class="status-cards">
                        <Show when=has_loading>
                            <Card>
                                <For
                                    each=move || handler.loading.get()
                                    key=|entry| entry.id
                                    children=|entry| view! {
                                        <p class="status-message">{entry.message}</p>
                                    }
                                />
                            </Card>
                        </Show>
                        <Show when=has_errors>
                            <Card
                                variant=CardVariant::Error
                                title=move || if all_dismissable() { "error" } else { "fatal error" }
                                on_close=Signal::derive(move ||
                                    all_dismissable().then(|| Callback::new(move |()| handler.dismiss_errors()))
                                )
                            >
                                <For
                                    each=move || handler.errors.get()
                                    key=|entry| entry.id
                                    children=|entry| view! {
                                        <p class="status-error-message">{entry.error}</p>
                                    }
                                />
                            </Card>
                        </Show>
                    </div>
                </div>
            </Portal>
            {move || children.with_value(|c| c())}
        </Provider>
    }
}
