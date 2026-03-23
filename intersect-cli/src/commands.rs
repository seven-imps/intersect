use std::str::FromStr;
use std::sync::{mpsc::SyncSender, Arc};

use intersect_core::{
    api::{Intersect, TypedReference},
    documents::AccountDocument,
    models::Trace,
    veilid::with_crypto,
};

use clap::Parser;

use crate::cli::{Cli, Commands, CreateCommands};

pub fn dispatch(input: &str, intersect: &Option<Arc<Intersect>>, tx: SyncSender<String>) {
    let args = match shlex::split(input) {
        Some(a) => a,
        None => {
            let _ = tx.send("error: invalid quoting".to_string());
            return;
        }
    };

    let cli = match Cli::try_parse_from(&args) {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(format!("{e}"));
            return;
        }
    };

    let Some(intersect) = intersect.clone() else {
        let _ = tx.send("error: not connected yet".to_string());
        return;
    };

    tokio::spawn(async move {
        execute(cli, intersect, tx).await;
    });
}

async fn execute(cli: Cli, intersect: Arc<Intersect>, tx: SyncSender<String>) {
    match cli.command {
        Commands::Create {
            what: CreateCommands::Account { name, bio },
        } => {
            let keypair = with_crypto(|c| c.generate_keypair());
            if let Err(e) = intersect.login(keypair) {
                let _ = tx.send(format!("login error: {e}"));
                return;
            }
            match intersect.create_account(name, bio, None).await {
                Ok(typed_ref) => {
                    let _ = tx.send("account created".to_string());
                    let _ = tx.send(format!("trace: {}", typed_ref.to_unlocked_trace()));
                }
                Err(e) => {
                    let _ = tx.send(format!("error: {e}"));
                }
            }
        }
        Commands::Open { trace } => {
            let trace = match Trace::from_str(&trace) {
                Ok(t) => t,
                Err(e) => {
                    let _ = tx.send(format!("invalid trace: {e}"));
                    return;
                }
            };
            let typed_ref = match TypedReference::<AccountDocument>::try_from(trace) {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.send(format!("trace error: {e}"));
                    return;
                }
            };
            match intersect.open(&typed_ref).await {
                Ok((_, rx)) => {
                    let view = match rx.borrow().as_ref() {
                        Ok(v) => v.clone(),
                        Err(e) => {
                            let _ = tx.send(format!("error: {e}"));
                            return;
                        }
                    };
                    let _ = tx.send(format!("name: {:?}", view.public.name()));
                    let _ = tx.send(format!("bio:  {:?}", view.public.bio()));
                }
                Err(e) => {
                    let _ = tx.send(format!("error: {e}"));
                }
            }
        }
    }
}
