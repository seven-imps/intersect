use std::sync::{Arc, mpsc};

use intersect_core::api::Intersect;

pub enum Status {
    Connecting,
    Ready,
    Failed(String),
    Closing,
}

pub struct App {
    pub input: String,
    pub log: Vec<String>,
    pub status: Status,
    pub intersect: Option<Arc<Intersect>>,
    pub cmd_tx: mpsc::SyncSender<String>,
}

impl App {
    pub fn is_closing(&self) -> bool {
        matches!(self.status, Status::Closing)
    }

    pub fn new(cmd_tx: mpsc::SyncSender<String>) -> Self {
        Self {
            input: String::new(),
            log: Vec::new(),
            status: Status::Connecting,
            intersect: None,
            cmd_tx,
        }
    }
}
