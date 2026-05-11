use std::{
    str::FromStr,
    sync::{
        mpsc::{Receiver, SyncSender},
        Arc, Mutex, OnceLock,
    },
};

use anyhow::{anyhow, Context};
use arboard::Clipboard;

use intersect_core::{documents::*, models::*, *};

use crate::{
    cli::{Cli, Commands, CreateCommands},
    prompt::{unlock_trace, Prompt},
    ui::panel::{AccountPanel, FragmentPanel, IndexPanel, LinksPanel, OpenPanel},
};

pub enum Output {
    Line(String),
    Error(String),
}

// thin wrapper to avoid `let _ = tx.send(Output::...)` noise everywhere
#[derive(Clone)]
pub struct Tx(SyncSender<Output>);

impl Tx {
    pub fn new_channel() -> (Self, Receiver<Output>) {
        let (cmd_tx, cmd_rx) = std::sync::mpsc::sync_channel::<Output>(64);
        (Self(cmd_tx), cmd_rx)
    }
    pub fn line(&self, s: impl Into<String>) {
        let _ = self.0.send(Output::Line(s.into()));
    }

    pub fn error(&self, s: impl Into<String>) {
        let _ = self.0.send(Output::Error(s.into()));
    }
}

pub async fn execute(
    cli: Cli,
    intersect: Arc<Intersect>,
    tx: Tx,
    panel_tx: SyncSender<OpenPanel>,
    prompt: &impl Prompt,
) {
    let result = match cli.command {
        Commands::Login { account, secret } => {
            cmd_login(account, secret, &intersect, &tx, prompt).await
        }
        Commands::Create {
            what:
                CreateCommands::Account {
                    name,
                    bio,
                    password,
                },
        } => cmd_create_account(name, bio, password, &intersect, &tx).await,
        Commands::Create {
            what:
                CreateCommands::Fragment {
                    path,
                    mime,
                    password,
                },
        } => cmd_create_fragment(path, mime, password, &intersect, &tx).await,
        Commands::Create {
            what:
                CreateCommands::Index {
                    name,
                    fragment,
                    links,
                    password,
                },
        } => cmd_create_index(name, fragment, links, password, &intersect, &tx).await,
        Commands::Fetch { trace, output } => {
            cmd_fetch(trace, output, &intersect, &tx, prompt).await
        }
        Commands::Open { trace } => cmd_open(trace, &intersect, &tx, &panel_tx, prompt).await,
        // handled at the ui layer before reaching here
        Commands::Exit => Ok(()),
    };
    if let Err(e) = result {
        tx.error(format!("{e:#}"));
    }
}

async fn cmd_login(
    account: Option<String>,
    secret: Option<String>,
    intersect: &Intersect,
    tx: &Tx,
    prompt: &impl Prompt,
) -> anyhow::Result<()> {
    let is_anon = account
        .as_deref()
        .is_none_or(|a| matches!(a, "anon" | "anonymous"));
    if is_anon {
        intersect.logout();
        tx.line("logged in anonymously");
        return Ok(());
    }
    let trace = Trace::from_str(account.as_deref().unwrap()).context("invalid trace")?;
    let secret = secret
        .ok_or_else(|| anyhow!("secret required for account login"))?
        .parse::<AccountSecret>()
        .context("invalid secret")?;
    let typed_ref = unlock_trace(trace.into_typed::<AccountDocument>()?, prompt).await?;
    intersect.login(typed_ref, secret).await?;
    tx.line("logged in");
    Ok(())
}

async fn cmd_create_account(
    name: Option<String>,
    bio: Option<String>,
    password: Option<String>,
    intersect: &Intersect,
    tx: &Tx,
) -> anyhow::Result<()> {
    let (typed_ref, secret) = intersect.create_account(name, bio, None).await?;
    tx.line("account created");
    print_trace(&typed_ref, password.as_deref(), tx)?;
    tx.line(format!("secret: {secret}"));
    Ok(())
}

async fn cmd_create_fragment(
    path: std::path::PathBuf,
    mime: String,
    password: Option<String>,
    intersect: &Intersect,
    tx: &Tx,
) -> anyhow::Result<()> {
    let data =
        std::fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let mime = FragmentMime::new(mime).context("invalid mime type")?;
    let typed_ref = intersect.create_fragment(data, mime).await?;
    tx.line("fragment created");
    print_trace(&typed_ref, password.as_deref(), tx)?;
    Ok(())
}

async fn cmd_create_index(
    name: String,
    fragment: Option<String>,
    links: Option<String>,
    password: Option<String>,
    intersect: &Intersect,
    tx: &Tx,
) -> anyhow::Result<()> {
    let parse_trace = |s: String| Trace::from_str(&s).context("invalid trace");
    let fragment = fragment.map(parse_trace).transpose()?;
    let links = links.map(parse_trace).transpose()?;
    let typed_ref = intersect.create_index(name, fragment, links).await?;
    tx.line("index created");
    print_trace(&typed_ref, password.as_deref(), tx)?;
    Ok(())
}

async fn cmd_fetch(
    trace: String,
    output: Option<std::path::PathBuf>,
    intersect: &Intersect,
    tx: &Tx,
    prompt: &impl Prompt,
) -> anyhow::Result<()> {
    let trace = Trace::from_str(&trace).context("invalid trace")?;
    match trace.document_type() {
        DocumentType::Fragment => {
            let r = unlock_trace(trace.into_typed::<FragmentDocument>()?, prompt).await?;
            let view = intersect.fetch(&r).await?;
            match output {
                Some(path) => {
                    std::fs::write(&path, view.data())
                        .with_context(|| format!("failed to write {}", path.display()))?;
                    tx.line(format!("written to {}", path.display()));
                }
                None => tx.line(format!("{view}")),
            }
        }
        DocumentType::Account => {
            let r = unlock_trace(trace.into_typed::<AccountDocument>()?, prompt).await?;
            let view = intersect.fetch(&r).await?;
            match output {
                Some(path) => {
                    std::fs::write(&path, format!("{view}"))
                        .with_context(|| format!("failed to write {}", path.display()))?;
                    tx.line(format!("written to {}", path.display()));
                }
                None => tx.line(format!("{view}")),
            }
        }
        DocumentType::Index => {
            let r = unlock_trace(trace.into_typed::<IndexDocument>()?, prompt).await?;
            let view = intersect.fetch(&r).await?;
            match output {
                Some(path) => {
                    std::fs::write(&path, format!("{view}"))
                        .with_context(|| format!("failed to write {}", path.display()))?;
                    tx.line(format!("written to {}", path.display()));
                }
                None => tx.line(format!("{view}")),
            }
        }
        DocumentType::Links => return Err(anyhow!("links documents are not yet supported")),
    }
    Ok(())
}

async fn cmd_open(
    trace: String,
    intersect: &Intersect,
    tx: &Tx,
    panel_tx: &SyncSender<OpenPanel>,
    prompt: &impl Prompt,
) -> anyhow::Result<()> {
    let trace = Trace::from_str(&trace).context("invalid trace")?;
    let panel = match trace.document_type() {
        DocumentType::Account => {
            let r = unlock_trace(trace.into_typed::<AccountDocument>()?, prompt).await?;
            let doc = intersect.open(&r).await?;
            OpenPanel::Account(AccountPanel { doc })
        }
        DocumentType::Index => {
            let r = unlock_trace(trace.into_typed::<IndexDocument>()?, prompt).await?;
            let doc = intersect.open(&r).await?;
            let (panel, errors) = IndexPanel::new(doc, intersect, prompt).await;
            for error in errors {
                tx.error(error);
            }
            OpenPanel::Index(panel)
        }
        DocumentType::Fragment => {
            let r = unlock_trace(trace.into_typed::<FragmentDocument>()?, prompt).await?;
            let view = intersect.fetch(&r).await?;
            OpenPanel::Fragment(FragmentPanel { view })
        }
        DocumentType::Links => return Err(anyhow!("links documents are not yet supported")),
    };
    let _ = panel_tx.send(panel);
    Ok(())
}

// ==== helpers ====

fn print_trace<D: Document>(
    typed_ref: &TypedReference<D>,
    password: Option<&str>,
    tx: &Tx,
) -> anyhow::Result<()> {
    let (trace, kind) = match password {
        Some(pw) => (typed_ref.to_protected_trace(pw)?, "trace (protected)"),
        None => (typed_ref.to_unlocked_trace(), "trace (unlocked)"),
    };
    let trace_str = trace.to_string();
    tx.line(format!("{kind}: {trace_str}"));
    copy_to_clipboard(&trace_str, tx);
    Ok(())
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
