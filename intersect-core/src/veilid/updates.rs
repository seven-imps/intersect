use std::sync::{Arc, Mutex};

use crate::debug;
use veilid_core::{
    VeilidAppCall, VeilidAppMessage, VeilidLog, VeilidRouteChange, VeilidStateAttachment,
    VeilidStateConfig, VeilidStateNetwork, VeilidUpdate, VeilidValueChange,
};

// most of this is shamelessly stolen from https://codeberg.org/cmars/veilnet/src/branch/main/src/connection/updates.rs
// thank you for the wonderful code <3

/// Handle veilid update callback events.
pub trait UpdateHandler {
    fn log(&self, _log: &VeilidLog) {}
    fn app_message(&self, _message: &VeilidAppMessage) {}
    fn app_call(&self, _call: &VeilidAppCall) {}
    fn state_attachment(&self, _attachment: &VeilidStateAttachment) {}
    fn state_network(&self, _network: &VeilidStateNetwork) {}
    fn state_config(&self, _config: &VeilidStateConfig) {}
    fn route_change(&self, _change: &VeilidRouteChange) {}
    fn value_change(&self, _change: &VeilidValueChange) {}
    fn shutdown(&self) {}
}

// just for convenience so we can use Arc<> for our handlers
impl<T: UpdateHandler> UpdateHandler for Arc<T> {
    fn log(&self, log: &VeilidLog) {
        self.as_ref().log(log)
    }
    fn app_message(&self, message: &VeilidAppMessage) {
        self.as_ref().app_message(message)
    }
    fn app_call(&self, call: &VeilidAppCall) {
        self.as_ref().app_call(call)
    }
    fn state_attachment(&self, attachment: &VeilidStateAttachment) {
        self.as_ref().state_attachment(attachment)
    }
    fn state_network(&self, network: &VeilidStateNetwork) {
        self.as_ref().state_network(network)
    }
    fn state_config(&self, config: &VeilidStateConfig) {
        self.as_ref().state_config(config)
    }
    fn route_change(&self, change: &VeilidRouteChange) {
        self.as_ref().route_change(change)
    }
    fn value_change(&self, change: &VeilidValueChange) {
        self.as_ref().value_change(change)
    }
    fn shutdown(&self) {
        self.as_ref().shutdown()
    }
}

/// Dispatch update callback events to an UpdateHandler.
pub struct UpdateDispatch {
    handler: Arc<Mutex<dyn UpdateHandler + Send + Sync>>,
}

impl UpdateDispatch {
    pub fn new(handler: Arc<Mutex<dyn UpdateHandler + Send + Sync>>) -> Self {
        Self { handler }
    }

    pub fn update(&self, update: VeilidUpdate) {
        let handler = self.handler.lock().unwrap();
        match update {
            VeilidUpdate::Log(ref veilid_log) => handler.log(veilid_log),
            VeilidUpdate::AppMessage(ref veilid_app_message) => {
                handler.app_message(veilid_app_message)
            }
            VeilidUpdate::AppCall(ref veilid_app_call) => handler.app_call(veilid_app_call),
            VeilidUpdate::Attachment(ref veilid_state_attachment) => {
                handler.state_attachment(veilid_state_attachment)
            }
            VeilidUpdate::Network(ref veilid_state_network) => {
                handler.state_network(veilid_state_network)
            }
            VeilidUpdate::Config(ref veilid_state_config) => {
                handler.state_config(veilid_state_config)
            }
            VeilidUpdate::RouteChange(ref veilid_route_change) => {
                handler.route_change(veilid_route_change)
            }
            VeilidUpdate::ValueChange(ref veilid_value_change) => {
                handler.value_change(veilid_value_change)
            }
            VeilidUpdate::Shutdown => handler.shutdown(),
        };
    }
}

/// Handler update event callbacks by invoking a chain of handlers.
pub struct HandlerChain {
    handlers: Vec<Box<dyn UpdateHandler + Send + Sync>>,
}

impl Default for HandlerChain {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerChain {
    pub fn new() -> Self {
        Self { handlers: vec![] }
    }

    pub fn add(&mut self, handler: Box<dyn UpdateHandler + Send + Sync>) {
        self.handlers.push(handler);
    }
}

impl UpdateHandler for HandlerChain {
    fn log(&self, log: &VeilidLog) {
        for handler in self.handlers.iter() {
            handler.log(log)
        }
    }
    fn app_message(&self, message: &VeilidAppMessage) {
        for handler in self.handlers.iter() {
            handler.app_message(message)
        }
    }
    fn app_call(&self, call: &VeilidAppCall) {
        for handler in self.handlers.iter() {
            handler.app_call(call)
        }
    }
    fn state_attachment(&self, attachment: &VeilidStateAttachment) {
        for handler in self.handlers.iter() {
            handler.state_attachment(attachment)
        }
    }
    fn state_network(&self, network: &VeilidStateNetwork) {
        for handler in self.handlers.iter() {
            handler.state_network(network)
        }
    }
    fn state_config(&self, config: &VeilidStateConfig) {
        for handler in self.handlers.iter() {
            handler.state_config(config)
        }
    }
    fn route_change(&self, change: &VeilidRouteChange) {
        for handler in self.handlers.iter() {
            handler.route_change(change)
        }
    }
    fn value_change(&self, change: &VeilidValueChange) {
        for handler in self.handlers.iter() {
            handler.value_change(change)
        }
    }
    fn shutdown(&self) {
        for handler in self.handlers.iter() {
            handler.shutdown()
        }
    }
}

/// Log all update events to the console.
pub struct UpdateLogger {}

impl UpdateLogger {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for UpdateLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateHandler for UpdateLogger {
    fn log(&self, _log: &VeilidLog) {
        // debug!("[veilid] log {:?}", log);
    }
    fn app_message(&self, message: &VeilidAppMessage) {
        debug!("[veilid] app message {:?}", message);
    }
    fn app_call(&self, call: &VeilidAppCall) {
        debug!("[veilid] app call {:?}", call);
    }
    fn state_attachment(&self, attachment: &VeilidStateAttachment) {
        debug!("[veilid] state attachment {:?}", attachment);
    }
    fn state_network(&self, _network: &VeilidStateNetwork) {
        // so noisy
        // debug!("[veilid] state network {:?}", network);
    }
    fn state_config(&self, config: &VeilidStateConfig) {
        debug!("[veilid] state config {:?}", config);
    }
    fn route_change(&self, change: &VeilidRouteChange) {
        debug!("[veilid] route change {:?}", change);
    }
    fn value_change(&self, change: &VeilidValueChange) {
        debug!("[veilid] value change {:?}", change);
    }
    fn shutdown(&self) {
        debug!("[veilid] shutdown");
    }
}
