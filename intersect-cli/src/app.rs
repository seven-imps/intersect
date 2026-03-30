use std::sync::{
    mpsc::{Receiver, SyncSender},
    Arc,
};

use intersect_core::api::Intersect;

use crate::commands::Output;

pub struct AppState {
    pub intersect: Option<Arc<Intersect>>,
    pub cmd_tx: SyncSender<Output>,
    pub cmd_rx: Receiver<Output>,
    pub stderr_rx: Receiver<String>,
    pub closing: bool,
}
