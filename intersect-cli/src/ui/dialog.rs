use std::sync::{Arc, Mutex};

use cursive::{
    view::{Nameable, Resizable},
    views::{Dialog, DummyView, Layer, LinearLayout, PaddedView, TextView},
    Cursive,
};

use super::AppState;

// adds a fullscreen backdrop + dialog as two layers, disabling command input capture
pub fn push_dialog(s: &mut Cursive, dialog: Dialog) {
    let state_arc = s.user_data::<Arc<Mutex<AppState>>>().unwrap().clone();
    {
        let mut state = state_arc.lock().unwrap();
        state.open_overlays += 1;
        state.sync_force_capture();
    }
    s.add_fullscreen_layer(Layer::new(DummyView.full_screen()));
    s.add_layer(dialog);
}

// removes the dialog and its backdrop, restoring input capture if no overlays remain
pub fn pop_dialog(s: &mut Cursive) {
    s.pop_layer();
    s.pop_layer();
    let state_arc = s.user_data::<Arc<Mutex<AppState>>>().unwrap().clone();
    {
        let mut state = state_arc.lock().unwrap();
        state.open_overlays = state.open_overlays.saturating_sub(1);
        state.sync_force_capture();
    }
}

pub fn animated_dialog(label: &str) -> Dialog {
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

// picks a dot frame based on current time, no state needed
pub fn dot_frame() -> &'static str {
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
