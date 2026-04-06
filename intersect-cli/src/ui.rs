use crate::cli::Cli;
use crate::{
    app::AppState,
    commands::{self, Output},
};
use clap::Parser;
use cursive::{
    event::{Callback, Event, EventResult, EventTrigger},
    theme::{BaseColor, BorderStyle, Color, ColorStyle, PaletteColor, PaletteStyle},
    view::{Nameable, Resizable, ScrollStrategy, Scrollable, ViewWrapper},
    views::{
        Dialog, DummyView, EditView, HideableView, Layer, LinearLayout, NamedView, PaddedView,
        Panel, ResizedView, ScrollView, TextView,
    },
    Cursive, Printer,
};
use intersect_core::{ConnectionStrength, NetworkState};
use std::sync::{Arc, Mutex, OnceLock};

static SPEED_FMT: OnceLock<numfmt::Formatter> = OnceLock::new();

// draws its child as if it always has focus — keeps the cursor visible
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

// adds a fullscreen backdrop + dialog as two layers
pub(crate) fn push_dialog(s: &mut Cursive, dialog: Dialog) {
    s.add_fullscreen_layer(Layer::new(DummyView.full_screen()));
    s.add_layer(dialog);
}

// removes the dialog and its backdrop
pub(crate) fn pop_dialog(s: &mut Cursive) {
    s.pop_layer();
    s.pop_layer();
}

fn animated_dialog(label: &str) -> Dialog {
    Dialog::around(PaddedView::lrtb(
        1,
        1,
        1,
        1,
        LinearLayout::horizontal()
            .child(TextView::new(label))
            .child(TextView::new("   ").with_name("anim-dots")),
    ))
}

// picks a dot frame based on current time — no state needed
fn dot_frame() -> &'static str {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    match (ms / 300) % 4 {
        0 => "   ",
        1 => ".  ",
        2 => ".. ",
        _ => "...",
    }
}

fn apply_theme(siv: &mut Cursive) {
    let mut theme = siv.current_theme().clone();
    theme.shadow = false;
    theme.borders = BorderStyle::Simple;
    use Color::TerminalDefault;
    use PaletteColor::*;
    theme.palette[Background] = TerminalDefault;
    theme.palette[Shadow] = TerminalDefault;
    theme.palette[View] = TerminalDefault;
    theme.palette[Primary] = TerminalDefault;
    // theme.palette[Primary] = Color::Dark(BaseColor::Yellow);
    theme.palette[Secondary] = TerminalDefault;
    theme.palette[Tertiary] = TerminalDefault;
    // theme.palette[TitlePrimary] = TerminalDefault;
    theme.palette[TitlePrimary] = Color::Dark(BaseColor::Yellow);
    theme.palette[TitleSecondary] = TerminalDefault;
    theme.palette[Highlight] = Color::Light(BaseColor::Yellow);
    theme.palette[HighlightInactive] = Color::Dark(BaseColor::Yellow);
    theme.palette[HighlightText] = Color::Dark(BaseColor::Black);
    theme.palette[PaletteStyle::EditableTextCursor] = ColorStyle::new(
        Color::Dark(BaseColor::Black),
        Color::Light(BaseColor::White),
    )
    .into();
    siv.set_theme(theme);
}

pub fn setup(siv: &mut Cursive) {
    apply_theme(siv);
    siv.set_fps(20);
    siv.add_global_callback(Event::Refresh, on_refresh);
    siv.clear_global_callbacks(Event::CtrlChar('c'));
    siv.set_on_pre_event(Event::CtrlChar('c'), on_ctrl_c);
    siv.set_on_pre_event('`', toggle_log); // pre-event so EditView doesn't eat it

    // typing a char while a scroll panel has focus snaps back to the input
    siv.set_on_pre_event_inner(
        EventTrigger::from_fn(|e| matches!(e, Event::Char(c) if *c != '`')),
        |event| {
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
    .max_height(10);

    let log_padding = HideableView::new(DummyView.fixed_height(1)).with_name("log-padding");

    let (status_left, status_right) = format_status_bar(None);
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
        ColorStyle::new(
            Color::Dark(BaseColor::Black),
            Color::Dark(BaseColor::Yellow),
        ),
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
                .child(TextView::new("> ").style(ColorStyle::front(Color::Dark(BaseColor::Yellow))))
                .child(AlwaysFocused(
                    EditView::new()
                        .filler(" ")
                        .style(ColorStyle::front(Color::Dark(BaseColor::Yellow)))
                        .on_submit(on_submit)
                        .with_name("input")
                        .full_width(),
                )),
        ));

    siv.add_fullscreen_layer(layout);

    siv.call_on_name("log-panel", |v: &mut LogPanel| v.set_visible(false));
    siv.call_on_name("log-padding", |v: &mut LogPadding| v.set_visible(false));

    push_dialog(siv, animated_dialog("initialising").title("intersect"));
}

fn on_refresh(s: &mut Cursive) {
    let state = s.user_data::<Arc<Mutex<AppState>>>().unwrap().clone();
    let state = state.lock().unwrap();

    let cmd_lines: Vec<_> = std::iter::from_fn(|| state.cmd_rx.try_recv().ok()).collect();
    let log_lines: Vec<_> = std::iter::from_fn(|| state.stderr_rx.try_recv().ok()).collect();
    let network = state
        .network_state_rx
        .as_ref()
        .map(|rx| rx.borrow().clone());
    drop(state);

    let (status_left, status_right) = format_status_bar(network.as_ref());
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
                        ColorStyle::front(Color::Light(BaseColor::Red)),
                    )),
                }
            }
        });
        s.call_on_name(
            "output-scroll",
            |v: &mut ScrollView<NamedView<TextView>>| {
                if v.is_at_bottom() {
                    let _ = v.set_scroll_strategy(ScrollStrategy::StickToBottom);
                }
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
            if v.is_at_bottom() {
                let _ = v.set_scroll_strategy(ScrollStrategy::StickToBottom);
            }
        });
    }

    // animate dots in connecting/closing dialog if one is present
    s.call_on_name("anim-dots", |v: &mut TextView| {
        v.set_content(dot_frame());
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
    let cmd_tx = state.cmd_tx.clone();
    drop(state);

    let args = match shlex::split(&text) {
        Some(a) => a,
        None => {
            let _ = cmd_tx.send(Output::Error("invalid quoting".to_string()));
            return;
        }
    };

    let cli = match Cli::try_parse_from(&args) {
        Ok(c) => c,
        Err(e) => {
            let _ = cmd_tx.send(Output::Error(format!("{e}")));
            return;
        }
    };

    if matches!(cli.command, crate::cli::Commands::Exit) {
        on_ctrl_c(s);
        return;
    }

    let Some(intersect) = intersect else {
        let _ = cmd_tx.send(Output::Error("not connected yet".to_string()));
        return;
    };

    tokio::spawn(async move {
        commands::execute(cli, intersect, cmd_tx).await;
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

    push_dialog(s, animated_dialog("shutting down").title("intersect"));

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

// returns (left, right) content for the status bar.
fn format_status_bar(network: Option<&NetworkState>) -> (String, String) {
    let prefix = format!("intersect │ v{}", env!("CARGO_PKG_VERSION"));

    let pending = network.map(|state| &state.pending_sync);

    let left = match pending {
        Some(p) if p.records > 0 => format!("{prefix} │ pending: {} ({})", p.records, p.subkeys),
        _ => prefix,
    };

    let network = network
        .map(format_network_state)
        .unwrap_or_else(|| "initialising...".into());

    (left, network)
}

fn format_network_state(state: &NetworkState) -> String {
    if !state.attached {
        match state.strength {
            ConnectionStrength::Attaching => "attaching...",
            ConnectionStrength::Detaching => "detaching...",
            ConnectionStrength::Detached => "detached",
            ConnectionStrength::Weak | ConnectionStrength::Good | ConnectionStrength::Strong => {
                "disconnected"
            }
        }
        .to_owned()
    } else {
        // format with three significant digits so things don't jump around
        let f = SPEED_FMT.get_or_init(|| "[~3b]B".parse().unwrap());
        let up_speed = f.fmt_string(state.bps_up);
        let down_speed = f.fmt_string(state.bps_down);

        let strength = match state.strength {
            ConnectionStrength::Weak => "■□□",
            ConnectionStrength::Good => "■■□",
            ConnectionStrength::Strong => "■■■",
            ConnectionStrength::Attaching
            | ConnectionStrength::Detaching
            | ConnectionStrength::Detached => "□□□",
        };

        format!("[{strength}] │ ↑ {} │ ↓ {}", up_speed, down_speed)
    }
}
