use std::str::FromStr;
use std::sync::{mpsc::SyncSender, Arc, Mutex, OnceLock};

use arboard::Clipboard;

use intersect_core::{
    models::{AccountSecret, FragmentMime, Trace},
    AccountDocument, FragmentDocument, Intersect, TypedReference,
};

use crate::cli::{Cli, Commands, CreateCommands};

pub enum Output {
    Line(String),
    Error(String),
}

// thin wrapper to avoid `let _ = tx.send(Output::...)` noise everywhere
struct Tx(SyncSender<Output>);

impl Tx {
    fn line(&self, s: impl Into<String>) {
        let _ = self.0.send(Output::Line(s.into()));
    }

    fn error(&self, s: impl Into<String>) {
        let _ = self.0.send(Output::Error(s.into()));
    }
}

pub async fn execute(cli: Cli, intersect: Arc<Intersect>, raw_tx: SyncSender<Output>) {
    let tx = Tx(raw_tx);
    match cli.command {
        Commands::Login { account, secret } => {
            let is_anon = account
                .as_deref()
                .is_none_or(|a| matches!(a, "anon" | "anonymous"));
            if is_anon {
                match intersect.login_anonymous() {
                    Ok(()) => tx.line("logged in anonymously"),
                    Err(e) => tx.error(format!("{e}")),
                }
            } else {
                let trace = match Trace::from_str(account.as_deref().unwrap()) {
                    Ok(t) => t,
                    Err(e) => {
                        tx.error(format!("invalid trace: {e}"));
                        return;
                    }
                };
                let secret = match secret.map(|s| s.parse::<AccountSecret>()) {
                    Some(Ok(s)) => s,
                    Some(Err(e)) => {
                        tx.error(format!("invalid secret: {e}"));
                        return;
                    }
                    None => {
                        tx.error("secret required for account login".to_string());
                        return;
                    }
                };
                let typed_ref = match TypedReference::<AccountDocument>::from_trace(trace) {
                    Ok(r) => r,
                    Err(e) => {
                        tx.error(format!("trace error: {e}"));
                        return;
                    }
                };
                match intersect.login(typed_ref, secret).await {
                    Ok(()) => tx.line("logged in"),
                    Err(e) => tx.error(format!("{e}")),
                }
            }
        }
        Commands::Create {
            what: CreateCommands::Account { name, bio },
        } => match intersect.create_account(name, bio, None).await {
            Ok((typed_ref, secret)) => {
                let trace = typed_ref.to_unlocked_trace();
                tx.line("account created");
                tx.line(format!("trace:  {trace}"));
                copy_to_clipboard(&trace.to_string(), &tx);
                tx.line(format!("secret: {secret}"));
            }
            Err(e) => tx.error(format!("{e}")),
        },
        Commands::Create {
            what: CreateCommands::Fragment { path, mime },
        } => {
            let data = match std::fs::read(&path) {
                Ok(d) => d,
                Err(e) => {
                    tx.error(format!("failed to read {}: {e}", path.display()));
                    return;
                }
            };
            let mime = match FragmentMime::new(mime) {
                Ok(m) => m,
                Err(e) => {
                    tx.error(format!("invalid mime type: {e}"));
                    return;
                }
            };
            match intersect.create_fragment(data, mime).await {
                Ok(typed_ref) => {
                    let trace = typed_ref.to_unlocked_trace();
                    tx.line("fragment created");
                    tx.line(format!("trace: {trace}"));
                    copy_to_clipboard(&trace.to_string(), &tx);
                }
                Err(e) => tx.error(format!("{e}")),
            }
        }
        Commands::Fetch { trace, output } => {
            let trace = match Trace::from_str(&trace) {
                Ok(t) => t,
                Err(e) => {
                    tx.error(format!("invalid trace: {e}"));
                    return;
                }
            };
            let typed_ref = match TypedReference::<FragmentDocument>::from_trace(trace) {
                Ok(r) => r,
                Err(e) => {
                    tx.error(format!("trace error: {e}"));
                    return;
                }
            };
            match intersect.fetch(&typed_ref).await {
                Ok(view) => {
                    if let Err(e) = std::fs::write(&output, view.data()) {
                        tx.error(format!("failed to write {}: {e}", output.display()));
                        return;
                    }
                    tx.line(format!("written to {}", output.display()));
                }
                Err(e) => tx.error(format!("{e}")),
            }
        }
        Commands::Open { trace } => {
            let trace = match Trace::from_str(&trace) {
                Ok(t) => t,
                Err(e) => {
                    tx.error(format!("invalid trace: {e}"));
                    return;
                }
            };
            let typed_ref = match TypedReference::<AccountDocument>::from_trace(trace) {
                Ok(r) => r,
                Err(e) => {
                    tx.error(format!("trace error: {e}"));
                    return;
                }
            };
            match intersect.open(&typed_ref).await {
                Ok(doc) => {
                    let view = match doc.updates.borrow().as_ref() {
                        Ok(v) => v.clone(),
                        Err(e) => {
                            tx.error(format!("{e}"));
                            return;
                        }
                    };
                    tx.line(format!("name: {:?}", view.public.name()));
                    tx.line(format!("bio:  {:?}", view.public.bio()));
                }
                Err(e) => tx.error(format!("{e}")),
            }
        }
    }
}

static CLIPBOARD: OnceLock<Option<Mutex<Clipboard>>> = OnceLock::new();

fn copy_to_clipboard(text: &str, tx: &Tx) {
    match CLIPBOARD.get_or_init(|| Clipboard::new().ok().map(Mutex::new)) {
        None => tx.line("(clipboard unavailable)"),
        Some(mutex) => match mutex.lock().unwrap().set_text(text) {
            Ok(_) => tx.line("(copied to clipboard)"),
            Err(e) => tx.line(format!("(clipboard unavailable: {e})")),
        },
    }
}
