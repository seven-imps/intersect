#![recursion_limit = "512"]

use std::{
    process::ExitCode,
    sync::{atomic::AtomicBool, Arc, Mutex},
};

use anyhow::Result;
use clap::Parser;
use cursive::{
    views::{Dialog, PaddedView, TextView},
    Cursive,
};
use intersect_core::{ConnectionParams, Intersect};

use crate::{
    commands::Tx,
    ui::{
        dialog::{pop_dialog, push_dialog},
        AppState,
    },
};

mod cli;
mod commands;
mod prompt;
mod stderr;
mod ui;

#[derive(Debug, Parser)]
#[command(name = "intersect")]
struct Args {
    /// run veilid node in a single-use namespace
    #[arg(short = 'e', long)]
    ephemeral: bool,

    /// run a single command instead of launching the tui
    #[arg(last = true)]
    command: Vec<String>,
}
#[tokio::main]
async fn main() -> Result<(), ExitCode> {
    let args = Args::parse();
    let connection_params = ConnectionParams {
        ephemeral: args.ephemeral,
    };

    if args.command.is_empty() {
        run_tui(connection_params).await
    } else {
        run_command(connection_params, args.command).await
    }
}

async fn run_tui(connection_params: ConnectionParams) -> Result<(), ExitCode> {
    let stderr_rx = stderr::capture();

    let (tx, rx) = Tx::new_channel();

    let mut siv = cursive::default();
    let cb_sink = siv.cb_sink().clone();
    let state = Arc::new(Mutex::new(AppState {
        intersect: None,
        network_state_rx: None,
        output_tx: tx,
        output_rx: rx,
        stderr_rx,
        closing: false,
        force_capture: Arc::new(AtomicBool::new(true)),
    }));
    siv.set_user_data(state.clone());
    ui::setup(&mut siv);

    // spin up intersect in a background task. it can go do its own thing over there
    let (cb, st) = (cb_sink.clone(), state.clone());
    tokio::spawn(async move {
        match Intersect::init(connection_params).await {
            Ok(i) => {
                let mut state = st.lock().unwrap();
                state.network_state_rx = Some(i.network_watch());
                state.intersect = Some(Arc::new(i));
                let _ = cb.send(Box::new(|s: &mut Cursive| {
                    pop_dialog(s);
                }));
            }
            Err(e) => {
                let msg = e.to_string();
                let _ = cb.send(Box::new(move |s: &mut Cursive| {
                    pop_dialog(s);
                    push_dialog(
                        s,
                        Dialog::around(PaddedView::lrtb(1, 1, 1, 1, TextView::new(msg)))
                            .title("connection failed")
                            .button("Quit", |s| s.quit()),
                    );
                }));
            }
        }
    });

    // and then run cursive in the main thread
    tokio::task::block_in_place(|| siv.run());

    Ok(())
}

async fn run_command(
    connection_params: ConnectionParams,
    command: Vec<String>,
) -> Result<(), ExitCode> {
    let cli = match cli::Cli::try_parse_from(&command) {
        Ok(c) => c,
        Err(e) => {
            eprint!("{e}");
            return Err(ExitCode::FAILURE);
        }
    };

    // init on the main thread since we wanna wait for it to spin up anyway
    let intersect = match Intersect::init(connection_params).await {
        Ok(i) => Arc::new(i),
        Err(e) => {
            eprintln!("error: {e}");
            return Err(ExitCode::FAILURE);
        }
    };

    // TODO: i think this isn't necessary anymore but i'm nervous to delete it
    // intersect.close _should_ wait for everything to flush but it's scary to rely on
    intersect.wait_for_attachment().await;

    let (output_tx, output_rx) = Tx::new_channel();
    commands::execute(cli, intersect.clone(), output_tx, &prompt::StdinPrompt).await;

    let mut errored = false;
    for msg in std::iter::from_fn(|| output_rx.try_recv().ok()) {
        match msg {
            commands::Output::Line(s) => println!("{s}"),
            commands::Output::Error(s) => {
                eprintln!("{s}");
                errored = true;
            }
        }
    }

    close(intersect).await;
    if errored {
        Err(ExitCode::FAILURE)
    } else {
        Ok(())
    }
}

pub(crate) async fn close(intersect: Arc<Intersect>) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
    let mut arc = intersect;
    loop {
        match Arc::try_unwrap(arc) {
            Ok(i) => {
                i.close().await;
                break;
            }
            Err(a) if std::time::Instant::now() < deadline => {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                arc = a;
            }
            Err(_) => break,
        }
    }
}
