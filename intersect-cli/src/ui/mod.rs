use crate::cli::Cli;
use crate::commands::{self, Output};
use clap::Parser;
use cursive::{
    event::{Callback, Event, EventResult, EventTrigger},
    theme::{ColorStyle, PaletteColor},
    view::{Nameable, Resizable, ScrollStrategy, Scrollable, ViewWrapper},
    views::{
        DummyView, EditView, HideableView, Layer, LinearLayout, NamedView, PaddedView, Panel,
        ResizedView, ScrollView, TextView,
    },
    Cursive, Printer,
};
use std::sync::{atomic::Ordering, Arc, Mutex};

pub mod dialog;
mod state;
pub mod status;
pub mod theme;
pub use state::AppState;

// draws its child as if it always has focus, keeps the cursor visible
// even when a scroll panel temporarily takes focus
struct AlwaysFocused<V>(V);

impl<V: cursive::View> ViewWrapper for AlwaysFocused<V> {
    type V = V;
    fn with_view<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&Self::V) -> R,
    {
        Some(f(&self.0))
    }
    fn with_view_mut<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Self::V) -> R,
    {
        Some(f(&mut self.0))
    }
    fn wrap_draw(&self, printer: &Printer) {
        let mut p = printer.clone();
        p.focused = true;
        p.enabled = true;
        self.with_view(|v| v.draw(&p));
    }
}
type LogPanel = HideableView<Panel<NamedView<ScrollView<NamedView<TextView>>>>>;
type LogPadding = HideableView<ResizedView<DummyView>>;

pub fn setup(siv: &mut Cursive) {
    let force_capture = siv
        .user_data::<Arc<Mutex<AppState>>>()
        .unwrap()
        .lock()
        .unwrap()
        .force_capture
        .clone();
    theme::apply_theme(siv);
    siv.set_fps(20);
    siv.add_global_callback(Event::Refresh, on_refresh);
    siv.clear_global_callbacks(Event::CtrlChar('c'));
    siv.set_on_pre_event(Event::CtrlChar('c'), on_ctrl_c);
    siv.set_on_pre_event('`', toggle_log); // pre-event so EditView doesn't eat it

    // typing a char while a scroll panel has focus snaps back to the command input.
    // skipped when force_capture is false (e.g. a text-input dialog is open).
    siv.set_on_pre_event_inner(
        EventTrigger::from_fn(|e| matches!(e, Event::Char(c) if *c != '`')),
        move |event| {
            if !force_capture.load(Ordering::Relaxed) {
                return None;
            }
            if let Event::Char(c) = *event {
                Some(EventResult::Consumed(Some(Callback::from_fn(move |s| {
                    s.focus_name("input").ok();
                    s.call_on_name("input", |v: &mut EditView| {
                        let _ = v.insert(c);
                    });
                }))))
            } else {
                None
            }
        },
    );

    let output = TextView::new("")
        .with_name("output")
        .scrollable()
        .scroll_strategy(ScrollStrategy::StickToBottom)
        .with_name("output-scroll")
        .full_screen();

    let log_hint =
        HideableView::new(TextView::new(" (press ` to toggle logs)")).with_name("log-hint");

    let log_panel = HideableView::new(
        Panel::new(
            TextView::new("")
                .with_name("log")
                .scrollable()
                .scroll_strategy(ScrollStrategy::StickToBottom)
                .with_name("log-scroll"),
        )
        .title("log"),
    )
    .with_name("log-panel")
    .max_height(12);

    let log_padding = HideableView::new(DummyView.fixed_height(1)).with_name("log-padding");

    let (status_left, status_right) = status::format_status_bar(None);
    let header = Layer::with_color(
        PaddedView::lrtb(
            1,
            1,
            0,
            0,
            LinearLayout::horizontal()
                .child(TextView::new(status_left).with_name("status-left"))
                .child(DummyView.full_width())
                .child(TextView::new(status_right).with_name("status-right")),
        )
        .full_width()
        .fixed_height(1),
        ColorStyle::new(PaletteColor::HighlightText, PaletteColor::HighlightInactive),
    );

    let layout = LinearLayout::vertical()
        .child(header)
        .child(DummyView.fixed_height(1))
        .child(log_hint)
        .child(log_panel)
        .child(log_padding)
        .child(Panel::new(output).title("output"))
        .child(PaddedView::lrtb(
            1,
            1,
            1,
            0,
            LinearLayout::horizontal()
                .child(TextView::new("> ").style(ColorStyle::front(PaletteColor::TitlePrimary)))
                .child(AlwaysFocused(
                    EditView::new()
                        .filler(" ")
                        .style(ColorStyle::front(PaletteColor::TitlePrimary))
                        .on_submit(on_submit)
                        .with_name("input")
                        .full_width(),
                )),
        ));

    siv.add_fullscreen_layer(layout);

    siv.call_on_name("log-panel", |v: &mut LogPanel| v.set_visible(false));
    siv.call_on_name("log-padding", |v: &mut LogPadding| v.set_visible(false));

    dialog::push_dialog(
        siv,
        dialog::animated_dialog("initialising").title("intersect"),
    );
}

fn on_refresh(s: &mut Cursive) {
    let state = s.user_data::<Arc<Mutex<AppState>>>().unwrap().clone();
    let state = state.lock().unwrap();

    let cmd_lines: Vec<_> = std::iter::from_fn(|| state.output_rx.try_recv().ok()).collect();
    let log_lines: Vec<_> = std::iter::from_fn(|| state.stderr_rx.try_recv().ok()).collect();
    let network = state
        .network_state_rx
        .as_ref()
        .map(|rx| rx.borrow().clone());
    drop(state);

    let (status_left, status_right) = status::format_status_bar(network.as_ref());
    s.call_on_name("status-left", |view: &mut TextView| {
        view.set_content(status_left)
    });
    s.call_on_name("status-right", |view: &mut TextView| {
        view.set_content(status_right)
    });

    if !cmd_lines.is_empty() {
        s.call_on_name("output", |v: &mut TextView| {
            for msg in cmd_lines {
                match msg {
                    Output::Line(s) => v.append(format!("{s}\n")),
                    Output::Error(s) => v.append(cursive::utils::markup::StyledString::styled(
                        format!("{s}\n"),
                        ColorStyle::front(theme::COLOR_ERROR),
                    )),
                }
            }
        });
        s.call_on_name(
            "output-scroll",
            |v: &mut ScrollView<NamedView<TextView>>| {
                // output always scrolls to the bottom on new content
                let _ = v.scroll_to_bottom();
            },
        );
    }
    if !log_lines.is_empty() {
        s.call_on_name("log", |v: &mut TextView| {
            for line in log_lines {
                v.append(format!("{line}\n"));
            }
        });
        s.call_on_name("log-scroll", |v: &mut ScrollView<NamedView<TextView>>| {
            // log only scrolls to the bottom is we're already there
            // this way a user can scroll up while logs are still flowing in
            // then go back to the bottom when thye're done and it'll snap to the latest logs again
            if v.is_at_bottom() {
                let _ = v.set_scroll_strategy(ScrollStrategy::StickToBottom);
            }
        });
    }

    // animate dots in connecting/closing dialog if one is present
    s.call_on_name("anim-dots", |v: &mut TextView| {
        v.set_content(dialog::dot_frame());
    });
}

fn on_submit(s: &mut Cursive, text: &str) {
    if text.trim().is_empty() {
        return;
    }
    let text = text.to_string();
    s.call_on_name("input", |v: &mut EditView| {
        v.set_content("");
    });
    s.call_on_name("output", |v: &mut TextView| {
        v.append(format!("> {text}\n"));
    });

    let state = s.user_data::<Arc<Mutex<AppState>>>().unwrap().clone();
    let state = state.lock().unwrap();
    let intersect = state.intersect.clone();
    let output_tx = state.output_tx.clone();
    drop(state);

    let args = match shlex::split(&text) {
        Some(a) => a,
        None => {
            output_tx.error("invalid quoting".to_string());
            return;
        }
    };

    let cli = match Cli::try_parse_from(&args) {
        Ok(c) => c,
        Err(e) => {
            output_tx.error(format!("{e}"));
            return;
        }
    };

    if matches!(cli.command, crate::cli::Commands::Exit) {
        on_ctrl_c(s);
        return;
    }

    let Some(intersect) = intersect else {
        output_tx.error("not connected yet".to_string());
        return;
    };

    let force_capture = {
        let state = s.user_data::<Arc<Mutex<AppState>>>().unwrap().clone();
        let guard = state.lock().unwrap();
        guard.force_capture.clone()
    };
    let prompt = crate::prompt::CursivePrompt {
        cb_sink: s.cb_sink().clone(),
        force_capture,
    };
    tokio::spawn(async move {
        commands::execute(cli, intersect, output_tx, &prompt).await;
    });
}

fn on_ctrl_c(s: &mut Cursive) {
    let state = s.user_data::<Arc<Mutex<AppState>>>().unwrap().clone();
    let mut state = state.lock().unwrap();

    if state.closing {
        // second ctrl+c: exit immediately without waiting
        drop(state);
        s.quit();
        return;
    }

    state.closing = true;
    let intersect = state.intersect.take();
    drop(state);

    dialog::push_dialog(
        s,
        dialog::animated_dialog("shutting down").title("intersect"),
    );

    let cb = s.cb_sink().clone();
    tokio::spawn(async move {
        if let Some(i) = intersect {
            crate::close(i).await;
        }
        let _ = cb.send(Box::new(|s: &mut Cursive| s.quit()));
    });
}

fn toggle_log(s: &mut Cursive) {
    s.call_on_name("log-hint", |v: &mut HideableView<TextView>| {
        v.set_visible(!v.is_visible());
    });
    s.call_on_name("log-panel", |v: &mut LogPanel| {
        v.set_visible(!v.is_visible());
    });
    s.call_on_name("log-padding", |v: &mut LogPadding| {
        v.set_visible(!v.is_visible());
    });
}
