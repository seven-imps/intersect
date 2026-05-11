mod account;
mod fragment;
mod index;
mod links;

pub use account::AccountPanel;
pub use fragment::FragmentPanel;
pub use index::IndexPanel;
pub use links::LinksPanel;

use std::sync::{Arc, Mutex};

use cursive::{
    event::Key,
    view::Resizable,
    views::{BoxedView, OnEventView, Panel as CursivePanel, ScrollView, StackView},
    Cursive,
};

use super::AppState;

pub trait Panel {
    fn title(&self) -> String;
    fn has_updates(&self) -> bool;
    /// builds the initial view tree; uses borrow_and_update to mark content seen
    fn build_view(&mut self, id: usize) -> Box<dyn cursive::View>;
    /// returns a closure that updates named subviews; called only when has_updates is true.
    /// borrow_and_update is called here (inside the lock), closure is applied after lock drops.
    fn make_update(&mut self, id: usize) -> Box<dyn FnOnce(&mut Cursive)>;
}

pub enum OpenPanel {
    Index(IndexPanel),
    Account(AccountPanel),
    Fragment(FragmentPanel),
    Links(LinksPanel),
}

// TODO: perhaps an enum was the wrong choice here, these matches seem silly
impl Panel for OpenPanel {
    fn title(&self) -> String {
        match self {
            Self::Index(p) => p.title(),
            Self::Account(p) => p.title(),
            Self::Fragment(p) => p.title(),
            Self::Links(p) => p.title(),
        }
    }

    fn has_updates(&self) -> bool {
        match self {
            Self::Index(p) => p.has_updates(),
            Self::Account(p) => p.has_updates(),
            Self::Fragment(p) => p.has_updates(),
            Self::Links(p) => p.has_updates(),
        }
    }

    fn build_view(&mut self, id: usize) -> Box<dyn cursive::View> {
        match self {
            Self::Index(p) => p.build_view(id),
            Self::Account(p) => p.build_view(id),
            Self::Fragment(p) => p.build_view(id),
            Self::Links(p) => p.build_view(id),
        }
    }

    fn make_update(&mut self, id: usize) -> Box<dyn FnOnce(&mut Cursive)> {
        match self {
            Self::Index(p) => p.make_update(id),
            Self::Account(p) => p.make_update(id),
            Self::Fragment(p) => p.make_update(id),
            Self::Links(p) => p.make_update(id),
        }
    }
}

pub struct PanelEntry {
    pub id: usize,
    pub panel: OpenPanel,
}

pub fn push(s: &mut Cursive, state_arc: &Arc<Mutex<AppState>>, mut panel: OpenPanel) {
    // assign id while locked
    let id = {
        let mut state = state_arc.lock().unwrap();
        let id = state.next_panel_id;
        state.next_panel_id += 1;
        id
    };

    // build initial view and title before moving panel into the stack.
    // build_view calls borrow_and_update so the first refresh doesn't re-render identical content.
    let title = panel.title();
    let view = panel.build_view(id);

    // push into stack, update force_capture (lock released after this block)
    {
        let mut state = state_arc.lock().unwrap();
        state.panel_stack.push(PanelEntry { id, panel });
        state.sync_force_capture();
    }

    // push into the content StackView
    s.call_on_name("content-stack", |stack: &mut StackView| {
        stack.add_fullscreen_layer(
            OnEventView::new(
                CursivePanel::new(ScrollView::new(BoxedView::new(view)))
                    .title(title)
                    .full_screen(),
            )
            .on_event('q', close_top)
            .on_event(Key::Esc, close_top),
        );
    });
    s.focus_name("content-stack").ok();
}

pub fn refresh(s: &mut Cursive, state_arc: &Arc<Mutex<AppState>>) {
    // produce the update closure inside the lock (borrow_and_update runs here),
    // then apply it after dropping the lock to avoid deadlock with cursive
    let f = {
        let mut state = state_arc.lock().unwrap();
        let entry = match state.panel_stack.last_mut() {
            Some(e) => e,
            None => return,
        };
        if !entry.panel.has_updates() {
            return;
        }
        entry.panel.make_update(entry.id)
    };
    f(s);
}

fn close_top(s: &mut Cursive) {
    // pop from state first, then from the StackView — mirrors the push order
    // (build inside lock, then call_on_name) and keeps them in sync if either fails
    let state_arc = s.user_data::<Arc<Mutex<AppState>>>().unwrap().clone();
    let panel_stack_empty = {
        let mut state = state_arc.lock().unwrap();
        state.panel_stack.pop();
        state.sync_force_capture();
        state.panel_stack.is_empty()
    };

    s.call_on_name("content-stack", |stack: &mut StackView| {
        stack.pop_layer();
    });

    if panel_stack_empty {
        s.focus_name("input").ok();
    }
}

/// constructs a stable name for a named subview within a panel
pub(crate) fn subview(id: usize, section: &str) -> String {
    format!("panel-{id}-{section}")
}
