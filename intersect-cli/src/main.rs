use std::sync::{Arc, Mutex};

use anyhow::Result;
use clap::Parser;
use cursive::{
    views::{Dialog, PaddedView, TextView},
    Cursive,
};
use intersect_core::{api::Intersect, veilid::ConnectionParams};

mod app;
mod cli;
mod commands;
mod stderr;
mod ui;

const CMD_CHANNEL_CAP: usize = 64;

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

fn main() -> Result<()> {
    let args = Args::parse();
    let connection_params = ConnectionParams {
        ephemeral: args.ephemeral,
    };

    if args.command.is_empty() {
        run_tui(connection_params)
    } else {
        run_command(connection_params, args.command)
    }
}

fn run_tui(connection_params: ConnectionParams) -> Result<()> {
    let stderr_rx = stderr::capture();
    let (cmd_tx, cmd_rx) = std::sync::mpsc::sync_channel::<commands::Output>(CMD_CHANNEL_CAP);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let mut siv = cursive::default();
    let cb_sink = siv.cb_sink().clone();
    let state = Arc::new(Mutex::new(app::AppState {
        intersect: None,
        cmd_tx,
        cmd_rx,
        stderr_rx,
        closing: false,
    }));
    siv.set_user_data(state.clone());
    ui::setup(&mut siv);

    let (cb, st) = (cb_sink.clone(), state.clone());
    rt.spawn(async move {
        match Intersect::init(connection_params).await {
            Ok(i) => {
                st.lock().unwrap().intersect = Some(Arc::new(i));
                let _ = cb.send(Box::new(|s: &mut Cursive| {
                    ui::pop_dialog(s);
                }));
            }
            Err(e) => {
                let msg = e.to_string();
                let _ = cb.send(Box::new(move |s: &mut Cursive| {
                    ui::pop_dialog(s);
                    ui::push_dialog(
                        s,
                        Dialog::around(PaddedView::lrtb(1, 1, 1, 1, TextView::new(msg)))
                            .title("connection failed")
                            .button("Quit", |s| s.quit()),
                    );
                }));
            }
        }
    });

    // enter the runtime so tokio::spawn works inside cursive callbacks
    let _guard = rt.enter();
    siv.run();
    drop(_guard);

    Ok(())
}

fn run_command(connection_params: ConnectionParams, command: Vec<String>) -> Result<()> {
    let cli = match cli::Cli::try_parse_from(&command) {
        Ok(c) => c,
        Err(e) => {
            eprint!("{e}");
            std::process::exit(2);
        }
    };

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        let intersect = match Intersect::init(connection_params).await {
            Ok(i) => Arc::new(i),
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        };

        let (cmd_tx, cmd_rx) = std::sync::mpsc::sync_channel(CMD_CHANNEL_CAP);
        commands::execute(cli, intersect.clone(), cmd_tx).await;

        let mut errored = false;
        for msg in std::iter::from_fn(|| cmd_rx.try_recv().ok()) {
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
            std::process::exit(1);
        }
    });

    Ok(())
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
