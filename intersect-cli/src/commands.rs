use std::str::FromStr;
use std::sync::{mpsc::SyncSender, Arc};

use intersect_core::{
    api::{Intersect, TypedReference},
    documents::AccountDocument,
    models::Trace,
    veilid::with_crypto,
};

use crate::cli::{Cli, Commands, CreateCommands};

pub enum Output {
    Line(String),
    Error(String),
}

pub async fn execute(cli: Cli, intersect: Arc<Intersect>, tx: SyncSender<Output>) {
    match cli.command {
        Commands::Create {
            what: CreateCommands::Account { name, bio },
        } => {
            let keypair = with_crypto(|c| c.generate_keypair());
            if let Err(e) = intersect.login(keypair) {
                let _ = tx.send(Output::Error(format!("login error: {e}")));
                return;
            }
            match intersect.create_account(name, bio, None).await {
                Ok(typed_ref) => {
                    let _ = tx.send(Output::Line("account created".to_string()));
                    let _ = tx.send(Output::Line(format!(
                        "trace: {}",
                        typed_ref.to_unlocked_trace()
                    )));
                }
                Err(e) => {
                    let _ = tx.send(Output::Error(format!("{e}")));
                }
            }
        }
        Commands::Open { trace } => {
            let trace = match Trace::from_str(&trace) {
                Ok(t) => t,
                Err(e) => {
                    let _ = tx.send(Output::Error(format!("invalid trace: {e}")));
                    return;
                }
            };
            let typed_ref = match TypedReference::<AccountDocument>::try_from(trace) {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.send(Output::Error(format!("trace error: {e}")));
                    return;
                }
            };
            match intersect.open(&typed_ref).await {
                Ok((_, rx)) => {
                    let view = match rx.borrow().as_ref() {
                        Ok(v) => v.clone(),
                        Err(e) => {
                            let _ = tx.send(Output::Error(format!("{e}")));
                            return;
                        }
                    };
                    let _ = tx.send(Output::Line(format!("name: {:?}", view.public.name())));
                    let _ = tx.send(Output::Line(format!("bio:  {:?}", view.public.bio())));
                }
                Err(e) => {
                    let _ = tx.send(Output::Error(format!("{e}")));
                }
            }
        }
    }
}
