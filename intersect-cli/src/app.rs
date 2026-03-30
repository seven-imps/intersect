use std::sync::{
    Arc,
    mpsc::{Receiver, SyncSender},
};

use intersect_core::api::Intersect;

pub struct AppState {
    pub intersect: Option<Arc<Intersect>>,
    pub cmd_tx: SyncSender<String>,
    pub cmd_rx: Receiver<String>,
    pub stderr_rx: Receiver<String>,
    pub closing: bool,
}
