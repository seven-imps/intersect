use std::sync::OnceLock;

use intersect_core::{ConnectionStrength, NetworkState};

static SPEED_FMT: OnceLock<numfmt::Formatter> = OnceLock::new();

// returns (left, right) content for the status bar.
pub fn format_status_bar(network: Option<&NetworkState>) -> (String, String) {
    let prefix = format!("intersect │ v{}", env!("CARGO_PKG_VERSION"));

    let pending = network.map(|state| &state.pending_sync);

    let left = match pending {
        Some(p) if p.records > 0 => format!("{prefix} │ pending: {} ({})", p.records, p.subkeys),
        _ => prefix,
    };

    let network = network
        .map(format_network_state)
        .unwrap_or_else(|| "initialising...".into());

    (left, network)
}

fn format_network_state(state: &NetworkState) -> String {
    if !state.attached {
        match state.strength {
            ConnectionStrength::Attaching => "attaching...",
            ConnectionStrength::Detaching => "detaching...",
            ConnectionStrength::Detached => "detached",
            ConnectionStrength::Weak | ConnectionStrength::Good | ConnectionStrength::Strong => {
                "disconnected"
            }
        }
        .to_owned()
    } else {
        // format with three significant digits so things don't jump around
        let f = SPEED_FMT.get_or_init(|| "[~3b]B".parse().unwrap());
        let up_speed = f.fmt_string(state.bps_up);
        let down_speed = f.fmt_string(state.bps_down);

        let strength = match state.strength {
            ConnectionStrength::Weak => "■□□",
            ConnectionStrength::Good => "■■□",
            ConnectionStrength::Strong => "■■■",
            ConnectionStrength::Attaching
            | ConnectionStrength::Detaching
            | ConnectionStrength::Detached => "□□□",
        };

        format!("[{strength}] │ ↑ {} │ ↓ {}", up_speed, down_speed)
    }
}
