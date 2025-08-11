use crate::log;
use async_once_cell::OnceCell;
use std::sync::{Arc, OnceLock};
use tracing_subscriber::prelude::*;
use tracing_subscriber::*;
use tracing_wasm::{WASMLayerConfigBuilder, *};
use veilid_core::{AttachmentState, CryptoSystemGuard, VeilidUpdate};
use veilid_core::{
    CryptoKind, RoutingContext, Sequencing, VeilidAPI, VeilidConfig, VeilidConfigBlockStore,
    VeilidConfigLogLevel, VeilidConfigProtectedStore, VeilidConfigTableStore,
};
use veilid_tools::Eventual;

pub const CRYPTO_KIND: CryptoKind = veilid_core::CRYPTO_KIND_VLD0;

pub async fn init() {
    VEILID.get_or_init(async { init_veilid().await }).await;
    // wait for the network to be up
    wait_for_network().await;
}

pub async fn shutdown() {
    VEILID.get().unwrap().clone().shutdown().await;
}

fn get_veilid() -> Option<VeilidAPI> {
    VEILID.get().cloned()
}

pub fn with_crypto<C: FnOnce(CryptoSystemGuard<'_>) -> T, T>(closure: C) -> T {
    let api = get_veilid().unwrap();
    let crypto = api.crypto().unwrap();
    let cs = crypto.get(CRYPTO_KIND).unwrap();

    closure(cs)
}

pub async fn get_routing_context() -> &'static RoutingContext {
    ROUTING_CONTEXT.get_or_init(|| {
        get_veilid()
            .unwrap()
            .routing_context()
            .unwrap()
            .with_sequencing(Sequencing::EnsureOrdered)
            .with_default_safety()
            .unwrap()
    })
}

// ======== //

static VEILID: OnceCell<VeilidAPI> = OnceCell::new();
static ROUTING_CONTEXT: OnceLock<RoutingContext> = OnceLock::new();
static NETWORK_READY: OnceLock<Eventual> = OnceLock::new();
static RELAY_READY: OnceLock<Eventual> = OnceLock::new();
static ATTACHED: OnceLock<Eventual> = OnceLock::new();

async fn init_veilid() -> VeilidAPI {
    let config = VeilidConfig {
        program_name: "intersect".into(),
        namespace: "intersect".into(),
        protected_store: VeilidConfigProtectedStore {
            // allow_insecure_fallback: true,
            directory: "./.veilid".into(),
            ..Default::default()
        },
        block_store: VeilidConfigBlockStore {
            directory: "./.veilid".into(),
            ..Default::default()
        },
        table_store: VeilidConfigTableStore {
            directory: "./.veilid".into(),
            ..Default::default()
        },
        ..Default::default()
    };

    // setup_logging();

    let update_callback = Arc::new(move |update: VeilidUpdate| veilid_callback(update));
    let veilid = veilid_core::api_startup_config(update_callback, config)
        .await
        .unwrap();

    NETWORK_READY.get_or_init(|| Eventual::new());
    RELAY_READY.get_or_init(|| Eventual::new());
    ATTACHED.get_or_init(|| Eventual::new());

    veilid.attach().await.unwrap();

    return veilid;
}

// #[cfg(target_arch = "wasm32")]
pub fn setup_wasm_logging() {
    // Set up subscriber and layers
    let subscriber = Registry::default();
    let mut layers = Vec::new();

    let ignore_list: Vec<String> = vec!["-veild_api", "-dht", "-fanout", "-network_result"]
        .into_iter()
        .map(|s| s.to_owned())
        .collect();

    let log_level = VeilidConfigLogLevel::Info;

    // Performance logger
    let filter = veilid_core::VeilidLayerFilter::new(log_level, &ignore_list, None);
    let layer = WASMLayer::new(
        WASMLayerConfigBuilder::new()
            .set_report_logs_in_timings(true)
            .set_console_config(ConsoleConfig::ReportWithConsoleColor)
            .build(),
    )
    .with_filter(filter.clone());
    layers.push(layer.boxed());

    // API logger
    let filter = veilid_core::VeilidLayerFilter::new(log_level, &ignore_list, None);
    let layer = veilid_core::ApiTracingLayer::init().with_filter(filter.clone());
    layers.push(layer.boxed());

    let subscriber = subscriber.with(layers);
    subscriber
        .try_init()
        .map_err(|e| format!("failed to initialize logging: {}", e))
        .expect("failed to initalize WASM platform");
}

fn veilid_callback(update: VeilidUpdate) {
    match update {
        VeilidUpdate::Network(_msg) => {
            // log!(
            //     "[veilid] network: Peers {:}, bytes/sec [{} up] [{} down]",
            //     msg.peers.iter().count(),
            //     msg.bps_up,
            //     msg.bps_down
            // )
        }
        VeilidUpdate::Attachment(msg) => {
            // update network state if necessary
            log!("[veilid] attachment: {:?}", msg);
            if msg.public_internet_ready {
                // log!("[veilid] internet ready");
                let _ = NETWORK_READY.get().unwrap().resolve();
            }
            if let AttachmentState::AttachedWeak
            | AttachmentState::AttachedGood
            | AttachmentState::AttachedStrong
            | AttachmentState::FullyAttached
            | AttachmentState::OverAttached = msg.state
            {
                // log!("[veilid] attached");
                let _ = ATTACHED.get().unwrap().resolve();
            }
        }

        VeilidUpdate::AppMessage(_msg) => {
            // log!("[veilid] message: {}", String::from_utf8_lossy(msg.message().into()));
        }
        VeilidUpdate::Log(msg) => {
            // log!("[veilid] log {:?}", msg);
            // this is only in a log message i guess??
            if msg.message.contains("[PublicInternet] set relay node") {
                log!("[veilid] relay ready");
                let _ = RELAY_READY.get().unwrap().resolve();
            }
        }
        VeilidUpdate::AppCall(_msg) => {
            // log!("[veilid] appcall {:?}", msg);
        }
        VeilidUpdate::Config(_msg) => {
            // log!("[veilid] config {:?}", msg);
        }
        VeilidUpdate::RouteChange(_msg) => {
            // log!("[veilid] route {:?}", msg);
        }
        VeilidUpdate::ValueChange(_msg) => {
            // log!("[veilid] value {:?}", msg);
        }
        VeilidUpdate::Shutdown => {
            log!("[veilid] shutdown");
        }
    };
}

async fn wait_for_network() {
    // log!("waiting for network...");
    NETWORK_READY.get().unwrap().instance_empty().await;
    // log!("waiting for relay...");
    // RELAY_READY.get().unwrap().instance_empty().await;
    // // log!("waiting for attachment...");
    ATTACHED.get().unwrap().instance_empty().await;
}
