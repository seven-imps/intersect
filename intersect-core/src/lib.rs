#![recursion_limit = "256"]

// internal modules
mod proto;
mod serialisation;
mod veilid;

// public modules
pub mod models;

// main api and documents is public, but re exported so it isn't nested
mod api;
pub use api::*;
mod documents;
pub use documents::*;

// and re-export anything from the internal modules that's used in the public api
pub use veilid::{ConnectionParams, ConnectionStrength, NetworkState};

/// platform agnostic logger
#[macro_export]
macro_rules! log {
    ($($tt:tt)*) => {
        $crate::_log(&format!($($tt)*), false)
    };
}

/// platform agnostic debug logger (only logs in debug builds)
#[macro_export]
macro_rules! debug {
    ($($tt:tt)*) => {
        #[cfg(debug_assertions)]
        $crate::_log(&format!($($tt)*), true)
    };
}

fn format_log(s: &str, debug: bool) -> String {
    if debug {
        format!("[DEBUG] {s}")
    } else {
        format!("[LOG] {s}")
    }
}

#[cfg(target_arch = "wasm32")]
pub fn _log(s: &str, debug: bool) {
    web_sys::console::log_1(&web_sys::wasm_bindgen::JsValue::from_str(&format_log(
        s, debug,
    )));
}

#[cfg(not(target_arch = "wasm32"))]
#[doc(hidden)]
#[allow(dead_code)]
pub fn _log(s: &str, debug: bool) {
    eprintln!("{}", format_log(s, debug));
}
