use tokio::sync::watch;
use veilid_core::{AttachmentState, ByteCount, VeilidStateAttachment, VeilidStateNetwork};

use crate::veilid::UpdateHandler;

pub struct NetworkWatcher {
    attachment_tx: watch::Sender<VeilidStateAttachment>,
    network_tx: watch::Sender<VeilidStateNetwork>,
}

impl NetworkWatcher {
    pub fn new() -> Self {
        let (attachment_tx, _) = watch::channel(default_attachment());
        let (network_tx, _) = watch::channel(default_network());
        Self {
            attachment_tx,
            network_tx,
        }
    }

    pub fn subscribe_attachment(&self) -> watch::Receiver<VeilidStateAttachment> {
        self.attachment_tx.subscribe()
    }

    pub fn subscribe_network(&self) -> watch::Receiver<VeilidStateNetwork> {
        self.network_tx.subscribe()
    }
}

impl Default for NetworkWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateHandler for NetworkWatcher {
    fn state_attachment(&self, attachment: &VeilidStateAttachment) {
        let _ = self.attachment_tx.send_replace(attachment.clone());
    }

    fn state_network(&self, network: &VeilidStateNetwork) {
        let _ = self.network_tx.send_replace(network.clone());
    }

    fn shutdown(&self) {
        let _ = self.attachment_tx.send_replace(default_attachment());
        let _ = self.network_tx.send_replace(default_network());
    }
}

fn default_attachment() -> VeilidStateAttachment {
    VeilidStateAttachment {
        state: AttachmentState::Detached,
        public_internet_ready: false,
        local_network_ready: false,
        uptime: 0.into(),
        attached_uptime: None,
    }
}

fn default_network() -> VeilidStateNetwork {
    VeilidStateNetwork {
        started: false,
        bps_down: ByteCount::from(0u64),
        bps_up: ByteCount::from(0u64),
        peers: vec![],
        node_ids: vec![],
    }
}
