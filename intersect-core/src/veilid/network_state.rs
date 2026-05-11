use futures::{FutureExt, select};
use tokio::sync::watch;
use veilid_core::{AttachmentState, VeilidStateAttachment, VeilidStateNetwork};
use veilid_tools::spawn::spawn_detached;

/// connection quality from the user's perspective.
/// collapses veilid's FullyAttached and OverAttached into Strong
#[derive(Clone, Debug, PartialEq)]
pub enum ConnectionStrength {
    Detached,
    Attaching,
    Detaching,
    Weak,
    Good,
    Strong,
}

/// number of records and subkeys not yet flushed to the DHT.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PendingSync {
    pub records: usize,
    pub subkeys: usize,
}

/// combined network status: veilid attachment/bandwidth state plus
/// the number of record subkeys not yet flushed to the DHT.
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkState {
    pub strength: ConnectionStrength,
    /// true when public_internet_ready and strength is at least Weak.
    /// this is the condition you want for "can i do network ops?".
    pub attached: bool,
    pub bps_down: u64,
    pub bps_up: u64,
    pub peer_count: usize,
    pub pending_sync: PendingSync,
}

/// checks whether a veilid attachment state represents a usable network connection..
pub fn is_attached(attachment: &VeilidStateAttachment) -> bool {
    attachment.public_internet_ready
        && matches!(
            attachment.state,
            AttachmentState::AttachedWeak
                | AttachmentState::AttachedGood
                | AttachmentState::AttachedStrong
                | AttachmentState::FullyAttached
                | AttachmentState::OverAttached
        )
}

fn connection_strength(state: &AttachmentState) -> ConnectionStrength {
    match state {
        AttachmentState::Detached => ConnectionStrength::Detached,
        AttachmentState::Attaching => ConnectionStrength::Attaching,
        AttachmentState::Detaching => ConnectionStrength::Detaching,
        AttachmentState::AttachedWeak => ConnectionStrength::Weak,
        AttachmentState::AttachedGood => ConnectionStrength::Good,
        AttachmentState::AttachedStrong
        | AttachmentState::FullyAttached
        | AttachmentState::OverAttached => ConnectionStrength::Strong,
    }
}

/// merges veilid attachment/network events and pool sync count into a single channel.
/// the returned receiver reflects the latest combined state and updates on any change.
pub fn watch_network_state(
    mut attachment_rx: watch::Receiver<VeilidStateAttachment>,
    mut network_rx: watch::Receiver<VeilidStateNetwork>,
    mut pending_sync_rx: watch::Receiver<PendingSync>,
) -> watch::Receiver<NetworkState> {
    // seed from current receiver values to avoid stale data getting stuck without being overwritten by an event
    let initial = {
        let attachment = attachment_rx.borrow();
        let network = network_rx.borrow();
        NetworkState {
            strength: connection_strength(&attachment.state),
            attached: is_attached(&attachment),
            bps_down: network.bps_down.as_u64(),
            bps_up: network.bps_up.as_u64(),
            peer_count: network.peers.len(),
            pending_sync: pending_sync_rx.borrow().clone(),
        }
    };
    let (tx, rx) = watch::channel(initial.clone());
    spawn_detached("watch_network_state", async move {
        loop {
            select! {
                result = attachment_rx.changed().fuse() => { if result.is_err() { break; } }
                result = network_rx.changed().fuse() => { if result.is_err() { break; } }
                result = pending_sync_rx.changed().fuse() => { if result.is_err() { break; } }
            }
            let attachment = attachment_rx.borrow_and_update();
            let network = network_rx.borrow_and_update();
            let new_state = NetworkState {
                strength: connection_strength(&attachment.state),
                attached: is_attached(&attachment),
                bps_down: network.bps_down.as_u64(),
                bps_up: network.bps_up.as_u64(),
                peer_count: network.peers.len(),
                pending_sync: pending_sync_rx.borrow_and_update().clone(),
            };
            drop((attachment, network));
            tx.send_if_modified(|current| {
                if *current == new_state {
                    return false;
                }
                // debug!("network state updated: {:?}", new_state)
                *current = new_state.clone();
                true
            });
        }
    });
    rx
}
