use std::sync::{Arc, mpsc};

use intersect_core::api::Intersect;
use ratatui::layout::Rect;

pub enum Status {
    Connecting,
    Ready,
    Failed(String),
    Closing,
}

pub struct App {
    pub input: String,
    pub output: Vec<String>,
    pub log: Vec<String>,
    pub log_expanded: bool,
    pub log_scroll: u16,  // lines from bottom (0 = auto-scroll to bottom)
    pub log_area: Rect,   // set each frame for mouse hit-testing
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
            output: Vec::new(),
            log: Vec::new(),
            log_expanded: false,
            log_scroll: 0,
            log_area: Rect::default(),
            status: Status::Connecting,
            intersect: None,
            cmd_tx,
        }
    }
}
