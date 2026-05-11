use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use anyhow::{anyhow, Context};
use intersect_core::{Document, TypedTrace, TypedReference};

use cursive::{
    view::Nameable,
    views::{Dialog, EditView, LinearLayout, PaddedView, TextView},
    Cursive,
};
use tokio::sync::oneshot;

/// to generically ask the user for a string
/// returns None if the user cancels.
pub trait Prompt: Send + Sync + 'static {
    async fn ask(&self, message: &str) -> Option<String>;
}

/// cli impl: reads from stdin with echo disabled
pub struct StdinPrompt;

impl Prompt for StdinPrompt {
    async fn ask(&self, message: &str) -> Option<String> {
        rpassword::prompt_password(message).ok()
    }
}

/// resolves an TypedTrace to a TypedReference, prompting for a password if needed
pub(crate) async fn unlock_trace<D: Document>(
    opened: TypedTrace<D>,
    prompt: &impl Prompt,
) -> anyhow::Result<TypedReference<D>> {
    match opened {
        TypedTrace::Unlocked(r) => Ok(r),
        TypedTrace::Locked(_) => Err(anyhow!(
            "locked traces (requiring a raw secret) are not yet supported"
        )),
        TypedTrace::Protected(protected_ref) => {
            let password = prompt
                .ask("password: ")
                .await
                .ok_or_else(|| anyhow!("cancelled"))?;
            protected_ref.unlock(&password).context("wrong password")
        }
    }
}

/// tui impl: pushes a cursive dialog and awaits the result via a oneshot channel
pub struct CursivePrompt {
    pub cb_sink: cursive::CbSink,
    pub force_capture: Arc<AtomicBool>,
}

impl Prompt for CursivePrompt {
    async fn ask(&self, message: &str) -> Option<String> {
        let (tx, rx) = oneshot::channel::<Option<String>>();
        // wrapped in Arc<Mutex<Option>> so both button callbacks can share ownership
        // of the sender while only the first one to fire actually sends
        let tx = Arc::new(Mutex::new(Some(tx)));

        let msg = message.to_string();
        let tx_ok = tx.clone();
        let tx_cancel = tx.clone();
        let fc_ok = self.force_capture.clone();
        let fc_cancel = self.force_capture.clone();

        self.force_capture.store(false, Ordering::Relaxed);
        if self
            .cb_sink
            .send(Box::new(move |s: &mut Cursive| {
                let dialog = Dialog::new()
                    .title(msg)
                    .content(PaddedView::lrtb(
                        1,
                        1,
                        1,
                        0,
                        LinearLayout::vertical()
                            .child(TextView::new("enter password:"))
                            .child(EditView::new().secret().with_name("prompt-input")),
                    ))
                    .button("ok", move |s| {
                        let pw = s
                            .call_on_name("prompt-input", |v: &mut EditView| v.get_content())
                            .unwrap();
                        s.pop_layer();
                        fc_ok.store(true, Ordering::Relaxed);
                        if let Some(tx) = tx_ok.lock().unwrap().take() {
                            let _ = tx.send(Some(pw.to_string()));
                        }
                    })
                    .button("cancel", move |s| {
                        s.pop_layer();
                        fc_cancel.store(true, Ordering::Relaxed);
                        if let Some(tx) = tx_cancel.lock().unwrap().take() {
                            let _ = tx.send(None);
                        }
                    });
                s.add_layer(dialog);
            }))
            .is_err()
        {
            // tui event loop is gone. restore capture and bail
            self.force_capture.store(true, Ordering::Relaxed);
            return None;
        }

        rx.await.ok().flatten()
    }
}
