use std::sync::{Arc, Mutex};

use anyhow::Result;
use cursive::{Cursive, views::{Dialog, PaddedView, TextView}};
use intersect_core::{api::Intersect, veilid::ConnectionParams};

mod app;
mod cli;
mod commands;
mod stderr;
mod ui;

fn main() -> Result<()> {
    let stderr_rx = stderr::capture();
    let (cmd_tx, cmd_rx) = std::sync::mpsc::sync_channel(64);

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
        match Intersect::init(ConnectionParams { ephemeral: false }).await {
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
