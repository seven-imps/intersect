use tokio::sync::watch;
use veilid_core::{AttachmentState, VeilidStateAttachment};

use crate::veilid::UpdateHandler;

// most of this is shamelessly stolen from https://codeberg.org/cmars/veilnet/src/branch/main/src/connection/updates.rs
// thank you for the wonderful code <3

/// Handle state attachment by distributing changes to a watch channel.
pub struct StateAttachmentWatcher {
    state_attachment_tx: watch::Sender<VeilidStateAttachment>,
}

impl StateAttachmentWatcher {
    /// Return a new handler and a receiver of changes to state attachment.
    pub fn new() -> (Self, watch::Receiver<VeilidStateAttachment>) {
        let (state_attachment_tx, state_attachment_rx) = watch::channel(default_state_attachment());
        (
            Self {
                state_attachment_tx,
            },
            state_attachment_rx,
        )
    }
}

impl UpdateHandler for StateAttachmentWatcher {
    fn state_attachment(&self, attachment: &VeilidStateAttachment) {
        let _ = self.state_attachment_tx.send_replace(attachment.clone());
    }
}

fn default_state_attachment() -> VeilidStateAttachment {
    VeilidStateAttachment {
        state: AttachmentState::Detached,
        public_internet_ready: false,
        local_network_ready: false,
        uptime: 0.into(),
        attached_uptime: None,
    }
}
