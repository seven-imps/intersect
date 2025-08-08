use std::future::Future;

use intersect_core::log;
use leptos::*;

use crate::components::Modal;

use super::empty_view;

#[derive(Clone, Copy)]
pub struct StatusContext {
    status_signal: RwSignal<Option<View>>,
    error_signal: RwSignal<String>,
}

impl StatusContext {
    pub fn new() -> Self {
        let status_signal = create_rw_signal(None);
        let error_signal = create_rw_signal("".to_string());

        StatusContext {
            status_signal,
            error_signal,
        }
    }

    pub fn set_status_view<S: FnOnce() -> View + 'static>(&self, status: S) {
        self.status_signal.set(Some(status()));
        self.clear_error();
    }

    pub fn set_status<S: ToString + 'static>(&self, status: S) {
        self.set_status_view(move || status.to_string().into_view())
    }

    pub fn set_error(&self, error: &anyhow::Error) {
        for e in error.chain() {
            log!("traceback: {}", e)
        }
        self.error_signal.set(error.to_string());
        self.clear_status();
    }

    pub fn clear_status(&self) {
        self.status_signal.set(None)
    }

    pub fn clear_error(&self) {
        self.error_signal.set("".to_owned())
    }

    pub fn clear(&self) {
        self.clear_status();
        self.clear_error();
    }

    pub fn run<F, O>(&self, function: F, message: Option<&str>) -> Result<O, anyhow::Error>
    where
        F: FnOnce() -> Result<O, anyhow::Error>,
    {
        if let Some(message) = message {
            let message = message.to_owned();
            self.set_status(message);
        }
        let result = function();
        match result {
            Result::Ok(_) => {
                if message.is_some() {
                    self.clear()
                }
            }
            Result::Err(ref error) => self.set_error(error),
        };

        result
    }

    pub async fn run_async<F, O, Fu>(&self, function: F, message: Option<&str>) -> Result<O, anyhow::Error>
    where
        F: FnOnce() -> Fu,
        Fu: Future<Output = Result<O, anyhow::Error>>,
    {
        if let Some(message) = message {
            let message = message.to_owned();
            self.set_status(message);
        }
        let result = function().await;
        match result {
            Result::Ok(_) => {
                if message.is_some() {
                    self.clear()
                }
            }
            Result::Err(ref error) => self.set_error(error),
        };

        result
    }
}

#[component]
pub fn Status(children: ChildrenFn) -> impl IntoView {
    let context: StatusContext = StatusContext::new();

    let status_view = move || {
        let status = context.status_signal.get();
        let error = context.error_signal.get();

        let should_show = status.is_some() || !error.is_empty();
        let show_error_modal = create_rw_signal(true);

        let error_memo = create_memo(move |_| error.clone());
        create_effect(move |_| {
            if !error_memo.get().is_empty() {
                show_error_modal.set(true);
            }
        });

        if let Some(status) = status {
            view! {
                <div class="status" aria-expanded= if should_show { "true" } else { "false" }>
                    <p class="status-text"> {status} </p>
                </div>
                <div class="status-backdrop"></div>
            }
            .into_view()
        } else if !error_memo.get().is_empty() {
            view! {
                <Modal show=show_error_modal title="An error occurred">
                    <p class="status-error-text"> {error_memo.get()} </p>
                </Modal>
            }
        } else {
            empty_view()
        }
    };

    view! {
        <Provider value=context>
            <Portal>
                {status_view}
            </Portal>
            {children}
        </Provider>
    }
}
