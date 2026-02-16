use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use thiserror::Error;

use tokio::sync::watch;
use veilid_core::{
    AttachmentState, CryptoKind, CryptoSystemGuard, VeilidAPI, VeilidConfig, VeilidStateAttachment,
    VeilidUpdate,
};

use crate::veilid::{
    StateAttachmentWatcher,
    updates::{HandlerChain, UpdateDispatch, UpdateHandler, UpdateLogger},
};

pub const CRYPTO_KIND: CryptoKind = veilid_core::CRYPTO_KIND_VLD0;

// most of this is shamelessly stolen from https://codeberg.org/cmars/veilnet/src/branch/main/src/connection/veilid/connection.rs
// thank you for the wonderful code <3

pub struct Connection {
    veilid: VeilidAPI,
    update_handlers: Arc<Mutex<HandlerChain>>,
    attachment_state_rx: watch::Receiver<VeilidStateAttachment>,
}

impl Connection {
    pub async fn init() -> Result<Self, ConnectionError> {
        // setup_logging();

        // set up the veilid event handler chain
        let update_handlers = Arc::new(Mutex::new(HandlerChain::new()));
        let update_source = Arc::new(UpdateDispatch::new(update_handlers.clone()));
        let update_callback = Arc::new(move |update: VeilidUpdate| {
            update_source.update(update);
        });

        let (attachment_watcher, attachment_state_rx) = StateAttachmentWatcher::new();

        // initialise the api
        let veilid = veilid_core::api_startup(update_callback, Self::config())
            .await
            .map_err(|e| ConnectionError::StartupFailed(e.to_string()))?;

        let connection = Self {
            veilid,
            update_handlers,
            attachment_state_rx,
        };

        // add default handlers
        connection.add_update_handler(Box::new(attachment_watcher));
        connection.add_update_handler(Box::new(UpdateLogger::new()));

        // boot up the node
        connection
            .veilid
            .attach()
            .await
            .map_err(|e| ConnectionError::StartupFailed(e.to_string()))?;

        Ok(connection)
    }

    fn config() -> VeilidConfig {
        let root_path = Path::new("./.intersect");
        VeilidConfig {
            program_name: "intersect".into(),
            namespace: "intersect".into(),
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

    pub fn add_update_handler(&self, handler: Box<dyn UpdateHandler + Send + Sync>) {
        self.update_handlers.lock().unwrap().add(handler);
    }

    /// Waits until the connection is attached and ready for public internet use.
    ///
    /// This blocks until the Veilid node is fully connected and can communicate
    /// over the public internet. Call this before performing network operations.
    pub async fn wait_for_attachment(&mut self) -> () {
        let is_attached = |attachment: &VeilidStateAttachment| {
            let attached = match attachment.state {
                AttachmentState::AttachedWeak
                | AttachmentState::AttachedGood
                | AttachmentState::AttachedStrong
                | AttachmentState::FullyAttached
                | AttachmentState::OverAttached => true,
                _ => false,
            };

            return attachment.public_internet_ready && attached;
        };
        self.attachment_state_rx
            .wait_for(is_attached)
            .await
            .unwrap(); // should never fail unless there's a logic error in the code
    }

    /// Gets the underlying Veilid routing context.
    pub fn routing_context(&self) -> veilid_core::RoutingContext {
        self.veilid.routing_context().unwrap() // it's fine
    }

    // /// Gets the underlying Veilid crypto context.
    // pub fn crypto(&self) -> VeilidComponentGuard<'_, Crypto> {
    //     self.veilid.crypto().unwrap() // don't worry kitten
    // }

    /// Executes a closure with access to the crypto system.
    pub fn with_crypto<T, F>(&self, f: F) -> T
    where
        F: Fn(CryptoSystemGuard<'_>) -> T,
    {
        let crypto_component = self.veilid.crypto().unwrap(); // don't worry kitten, it's fine.
        let crypto_system = crypto_component.get(CRYPTO_KIND).unwrap(); // the unwraps can't hurt you.
        f(crypto_system)
    }

    /// Closes the connection and cleans up resources.
    pub async fn close(self) -> () {
        self.veilid.shutdown().await;
    }
}

#[derive(Error, Debug, Clone)]
#[non_exhaustive]
pub enum ConnectionError {
    #[error("veilid startup failed: {0}")]
    StartupFailed(String),
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
