mod api;
mod keys;
pub mod models;
pub mod record;
pub mod rw_helpers;
mod veilid;

pub use api::*;
pub use keys::*;
pub use veilid::get_routing_context;
pub use veilid::setup_wasm_logging;
pub use veilid_core::ValueSubkey;

pub async fn init() {
    veilid::init().await;
}

pub async fn shutdown() {
    println!("shutting down...");
    veilid::shutdown().await;
}

// platform agnostic logging helper
#[macro_export]
macro_rules! log {
    ($($tt:tt)*) => {
        $crate::_log(&format!($($tt)*))
    };
}

fn format_log(s: &str) -> String {
    format!("[isec] {s}")
}

#[cfg(target_arch = "wasm32")]
pub fn _log(s: &str) {
    web_sys::console::log_1(&web_sys::wasm_bindgen::JsValue::from_str(&format_log(s)));
}

#[cfg(not(target_arch = "wasm32"))]
#[doc(hidden)]
#[allow(dead_code)]
pub fn _log(s: &str) {
    eprintln!("{}", format_log(s));
}
