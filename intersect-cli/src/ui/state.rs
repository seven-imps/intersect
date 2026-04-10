use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::Receiver,
    mpsc::SyncSender,
    Arc,
};

use intersect_core::{Intersect, NetworkState};
use tokio::sync::watch;

use crate::commands::{Output, Tx};

use super::panel::{OpenPanel, PanelEntry};

pub struct AppState {
    pub intersect: Option<Arc<Intersect>>,
    pub network_state_rx: Option<watch::Receiver<NetworkState>>,
    pub output_tx: Tx,
    pub output_rx: Receiver<Output>,
    pub stderr_rx: Receiver<String>,
    pub closing: bool,
    // when false, the global char pre-event passes events through
    // instead of capturing them into the command input.
    // true only when no dialogs and no panels are open.
    pub force_capture: Arc<AtomicBool>,
    // counts open global overlays to help determine if we should capture input
    pub open_overlays: usize,
    pub panel_stack: Vec<PanelEntry>,
    pub panel_tx: SyncSender<OpenPanel>,
    pub panel_rx: Receiver<OpenPanel>,
    pub next_panel_id: usize,
}

impl AppState {
    /// recomputes and stores force_capture based on current overlay + panel state
    pub fn sync_force_capture(&self) {
        let should_capture = self.open_overlays == 0 && self.panel_stack.is_empty();
        self.force_capture.store(should_capture, Ordering::Relaxed);
    }
}
