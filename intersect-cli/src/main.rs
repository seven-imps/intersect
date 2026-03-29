use std::sync::{
    mpsc::{self, Receiver},
    Arc,
};

use anyhow::Result;
use intersect_core::{
    api::{Intersect, IntersectError},
    veilid::ConnectionParams,
};
use ratatui::crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture,
        EnableBracketedPaste, EnableMouseCapture,
        Event, KeyCode, KeyModifiers, MouseEventKind,
    },
    execute,
};
use tokio::sync::oneshot;

mod app;
mod cli;
mod commands;
mod stderr;
mod ui;

#[tokio::main]
async fn main() -> Result<()> {
    let stderr_rx = stderr::capture();

    let (tx, init_rx) = oneshot::channel::<Result<Intersect, IntersectError>>();
    tokio::spawn(async move {
        let _ = tx.send(Intersect::init(ConnectionParams { ephemeral: false }).await);
    });

    let (cmd_tx, cmd_rx) = mpsc::sync_channel(64);
    let (panic_tx, panic_rx) = mpsc::sync_channel(64);
    let mut app = app::App::new(cmd_tx);
    ratatui::run(|terminal| run(terminal, &mut app, init_rx, stderr_rx, cmd_rx, panic_tx, panic_rx))
}

fn run(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut app::App,
    init_rx: oneshot::Receiver<Result<Intersect, IntersectError>>,
    stderr_rx: Receiver<String>,
    cmd_rx: Receiver<String>,
    panic_tx: mpsc::SyncSender<String>,
    panic_rx: Receiver<String>,
) -> Result<()> {
    // replace default paste handler so we can handle pastes as a single event rather than a stream of key events
    execute!(std::io::stdout(), EnableBracketedPaste)?;

    // override ratatui's panic hook
    // worker-thread panics (e.g. from spawned commands) should show in the log, not close the ui
    // TODO: show panics in a modal (they "should never happen" but are useful to surface clearly)
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if std::thread::current().name().is_some_and(|n| n == "main") {
            prev_hook(info); // restores terminal and exits
        }
        let _ = panic_tx.try_send(format!("panic: {info}"));
    }));

    let result = event_loop(terminal, app, init_rx, stderr_rx, cmd_rx, panic_rx);

    let _ = std::panic::take_hook(); // restore the original panic handler
    execute!(std::io::stdout(), DisableBracketedPaste)?; // restore default paste handling
    result
}

fn event_loop(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut app::App,
    mut init_rx: oneshot::Receiver<Result<Intersect, IntersectError>>,
    stderr_rx: Receiver<String>,
    cmd_rx: Receiver<String>,
    panic_rx: Receiver<String>,
) -> Result<()> {
    let mut close_rx: Option<oneshot::Receiver<()>> = None;

    loop {
        while let Ok(line) = stderr_rx.try_recv() {
            app.log.push(line);
        }
        while let Ok(line) = panic_rx.try_recv() {
            app.log.push(line);
        }
        while let Ok(line) = cmd_rx.try_recv() {
            app.output.push(line);
        }

        match init_rx.try_recv() {
            Ok(Ok(intersect)) if app.is_closing() => {
                // init finished while we were already closing: close it right away
                close_rx = Some(spawn_close(Arc::new(intersect)));
            }
            Ok(Ok(intersect)) => {
                app.intersect = Some(Arc::new(intersect));
                app.status = app::Status::Ready;
            }
            Ok(Err(_)) if app.is_closing() => return Ok(()),
            Ok(Err(e)) => app.status = app::Status::Failed(e.to_string()),
            Err(_) => {}
        }

        if close_rx.as_mut().is_some_and(|rx| rx.try_recv().is_ok()) {
            return Ok(());
        }

        terminal.draw(|f| ui::render(f, app))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            match event::read()? {
                Event::Paste(text) if !app.is_closing() => {
                    app.input.push_str(&text.replace(['\n', '\r'], ""));
                }
                Event::Key(key) => match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if app.is_closing() {
                            return Ok(()); // second ctrl+c: skip waiting, ratatui cleans up
                        }
                        app.status = app::Status::Closing;
                        app.input.clear();
                        if let Some(intersect) = app.intersect.take() {
                            close_rx = Some(spawn_close(intersect));
                        }
                        // if still connecting, stay in the loop, init completion handled above
                    }
                    _ if !app.is_closing() => match key.code {
                        KeyCode::Char('`') | KeyCode::Char('~') => {
                            app.log_expanded = !app.log_expanded;
                        }
                        KeyCode::Char(c) => app.input.push(c),
                        KeyCode::Backspace => {
                            app.input.pop();
                        }
                        KeyCode::Enter => {
                            let input: String = app.input.drain(..).collect();
                            if !input.is_empty() {
                                app.output.clear();
                                app.output.push(format!("> {input}"));
                                commands::dispatch(&input, &app.intersect, app.cmd_tx.clone());
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

fn spawn_close(intersect: Arc<Intersect>) -> oneshot::Receiver<()> {
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        // command tasks may briefly hold arc refs. retry until we can unwrap or timeout
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
        let _ = tx.send(());
    });
    rx
}
