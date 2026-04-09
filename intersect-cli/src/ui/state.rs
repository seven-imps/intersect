use std::sync::{atomic::AtomicBool, mpsc::Receiver, Arc};

use intersect_core::{Intersect, NetworkState};
use tokio::sync::watch;

use crate::commands::{Output, Tx};

pub struct AppState {
    pub intersect: Option<Arc<Intersect>>,
    pub network_state_rx: Option<watch::Receiver<NetworkState>>,
    pub output_tx: Tx,
    pub output_rx: Receiver<Output>,
    pub stderr_rx: Receiver<String>,
    pub closing: bool,
    // when false, the global char pre-event passes events through
    // instead of capturing them into the command input
    pub force_capture: Arc<AtomicBool>,
}
