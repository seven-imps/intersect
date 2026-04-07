use std::sync::{Arc, Mutex, OnceLock};
use thiserror::Error;

use tokio::sync::watch;
use veilid_core::{
    CryptoKind, CryptoSystemGuard, PublicKey, VeilidAPI, VeilidConfig, VeilidStateAttachment,
    VeilidStateNetwork, VeilidUpdate,
};

use crate::veilid::{
    NetworkWatcher, is_attached,
    updates::{HandlerChain, UpdateDispatch, UpdateHandler, UpdateLogger},
};

pub const CRYPTO_KIND: CryptoKind = veilid_core::CRYPTO_KIND_VLD0;

#[derive(Debug, Clone)]
pub struct ConnectionParams {
    pub ephemeral: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for ConnectionParams {
    fn default() -> Self {
        Self { ephemeral: false }
    }
}

// any network ops will go though Connection, but we do want this static global
// just so we can build the with_crypto helper below and avoid passing the crypto system around everywhere
static VEILID: OnceLock<VeilidAPI> = OnceLock::new();

/// executes a closure with access to the crypto system.
///
/// trade-off:
/// `VEILID` is a process-wide singleton set on first `Connection::init`. this avoids threading
/// a `VeilidAPI` handle through every crypto call site, but it means calling `with_crypto`
/// after `Connection::close` (which calls `veilid.shutdown()`) will panic.
pub fn with_crypto<T, F>(f: F) -> T
where
    F: Fn(CryptoSystemGuard<'_>) -> T,
{
    let veilid = VEILID.get().expect("Veilid API not initialized");
    let crypto_component = veilid.crypto().unwrap(); // don't worry kitten, it's fine.
    let crypto_system = crypto_component.get(CRYPTO_KIND).unwrap(); // the unwraps can't hurt you.
    f(crypto_system)
}

// most of this is shamelessly stolen from https://codeberg.org/cmars/veilnet/src/branch/main/src/connection/veilid/connection.rs
// thank you for the wonderful code <3

#[derive(Clone)] // cloneable cause all fields are Arc<Mutex<>> (VeilidAPI is internally Arc<Mutex<>>)
pub struct Connection {
    veilid: VeilidAPI,
    update_handlers: Arc<Mutex<HandlerChain>>,
    network_watcher: Arc<NetworkWatcher>,
}

impl Connection {
    pub async fn init(params: ConnectionParams) -> Result<Self, ConnectionError> {
        // setup_logging();
        // set up the veilid event handler chain
        let update_handlers = Arc::new(Mutex::new(HandlerChain::new()));
        let update_source = Arc::new(UpdateDispatch::new(update_handlers.clone()));
        let update_callback = Arc::new(move |update: VeilidUpdate| {
            update_source.update(update);
        });

        let network_watcher = Arc::new(NetworkWatcher::new());

        // initialise the api
        let veilid = veilid_core::api_startup(update_callback, Self::config(params))
            .await
            .map_err(|e| ConnectionError::StartupFailed(e.to_string()))?;
        VEILID.get_or_init(|| veilid.clone());

        let connection = Self {
            veilid,
            update_handlers,
            network_watcher,
        };

        // add default handlers
        connection.add_update_handler(Box::new(Arc::clone(&connection.network_watcher)));
        connection.add_update_handler(Box::new(UpdateLogger::default()));

        Ok(connection)
    }

    /// start connecting to the network.
    /// separated from init so we can initialise veilid without the network for tests and such.
    pub async fn attach(&self) -> Result<(), ConnectionError> {
        self.veilid
            .attach()
            .await
            .map_err(|e| ConnectionError::StartupFailed(e.to_string()))
    }

    fn config(params: ConnectionParams) -> VeilidConfig {
        use std::path::Path;
        let namespace = if params.ephemeral {
            format!("intersect-{:x}", rand::random::<u64>())
        } else {
            "intersect".into()
        };
        let root_path = Path::new("./.intersect");
        VeilidConfig {
            program_name: "intersect".into(),
            namespace,
            protected_store: veilid_core::VeilidConfigProtectedStore {
                // allow_insecure_fallback: true,
                directory: root_path.join("protected").to_string_lossy().to_string(),
                ..Default::default()
            },
            block_store: veilid_core::VeilidConfigBlockStore {
                directory: root_path.join("block").to_string_lossy().to_string(),
                ..Default::default()
            },
            table_store: veilid_core::VeilidConfigTableStore {
                directory: root_path.join("table").to_string_lossy().to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Closes the connection and cleans up resources.
    pub async fn close(self) -> () {
        self.veilid.shutdown().await;
    }

    pub fn add_update_handler(&self, handler: Box<dyn UpdateHandler + Send + Sync>) {
        self.update_handlers.lock().unwrap().add(handler);
    }

    /// returns a receiver for veilid attachment state changes
    pub fn attachment_state(&self) -> watch::Receiver<VeilidStateAttachment> {
        self.network_watcher.subscribe_attachment()
    }

    /// returns a receiver for veilid network state changes
    pub fn network_state(&self) -> watch::Receiver<VeilidStateNetwork> {
        self.network_watcher.subscribe_network()
    }

    /// blocks until the node is attached and ready for public internet use
    pub async fn wait_for_attachment(&self) {
        self.network_watcher
            .subscribe_attachment()
            .wait_for(is_attached)
            .await
            .unwrap();
    }

    /// Gets the underlying Veilid routing context.
    /// Returns an error if the connection has already been shut down or hasn't started yet.
    pub fn routing_context(&self) -> Result<veilid_core::RoutingContext, ConnectionError> {
        self.veilid
            .routing_context()
            .map_err(|_| ConnectionError::NoRoutingContext)
    }

    pub fn generate_member_id(&self, key: &PublicKey) -> veilid_core::MemberId {
        self.veilid.generate_member_id(key).unwrap()
    }
}

#[derive(Error, Debug, Clone)]
#[non_exhaustive]
pub enum ConnectionError {
    #[error("veilid startup failed: {0}")]
    StartupFailed(String),

    #[error("no routing context")]
    NoRoutingContext,
}

// // #[cfg(target_arch = "wasm32")]
// pub fn setup_wasm_logging() {
//     // Set up subscriber and layers
//     let subscriber = Registry::default();
//     let mut layers = Vec::new();

//     let ignore_list: Vec<String> = vec!["-veild_api", "-dht", "-fanout", "-network_result"]
//         .into_iter()
//         .map(|s| s.to_owned())
//         .collect();

//     let log_level = VeilidConfigLogLevel::Info;

//     // Performance logger
//     let filter = veilid_core::VeilidLayerFilter::new(log_level, &ignore_list);
//     let layer = WASMLayer::new(
//         WASMLayerConfigBuilder::new()
//             .set_report_logs_in_timings(true)
//             .set_console_config(ConsoleConfig::ReportWithConsoleColor)
//             .build(),
//     )
//     .with_filter(filter.clone());
//     layers.push(layer.boxed());

//     // API logger
//     let filter = veilid_core::VeilidLayerFilter::new(log_level, &ignore_list);
//     let layer = veilid_core::ApiTracingLayer::init().with_filter(filter.clone());
//     layers.push(layer.boxed());

//     let subscriber = subscriber.with(layers);
//     subscriber
//         .try_init()
//         .map_err(|e| format!("failed to initialize logging: {}", e))
//         .expect("failed to initalize WASM platform");
// }
