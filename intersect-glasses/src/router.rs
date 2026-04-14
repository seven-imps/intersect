use intersect_core::log;
use lazy_regex::{Lazy, Regex, lazy_regex};
use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::pages::{HomePage, TracePage};

// matches #/<path> or #/<path>?<args>
pub static ROUTE_REGEX: Lazy<Regex> = lazy_regex!(r"#/(?<path>[^?]*)(\?(?<args>.*))?$");

/// all app routes with their associated arguments
/// (currently only a single string arg is supported)
#[derive(Clone, PartialEq, Debug)]
pub enum AppRoute {
    Home,
    Trace(String),
    Login,
    NewPost,
    Account,
    NotFound,
}

impl AppRoute {
    /// parse a route from a path and args string.
    /// strips a leading "#/" if present, so callers can pass either form.
    pub fn parse(path: &str, args: String) -> Self {
        let path = path.strip_prefix("#/").unwrap_or(path);
        match path {
            "" | "home" => AppRoute::Home,
            "trace" => AppRoute::Trace(args),
            "login" => AppRoute::Login,
            "new" => AppRoute::NewPost,
            "account" => AppRoute::Account,
            _ => AppRoute::NotFound,
        }
    }

    /// route to url mapping
    /// if `absolute` is true, prefixes with "#/" for use in href attributes.
    pub fn href(&self, absolute: bool) -> String {
        let path = match self {
            AppRoute::Home => "".to_string(),
            AppRoute::Trace(key) => format!("trace?{key}"),
            AppRoute::Login => "login".to_string(),
            AppRoute::NewPost => "new".to_string(),
            AppRoute::Account => "account".to_string(),
            AppRoute::NotFound => "nothing".to_string(),
        };
        if absolute { format!("#/{path}") } else { path }
    }
}

#[component]
pub fn HashRouter() -> impl IntoView {
    log!("router initialised");
    let hash = use_location().hash;
    let state = use_location().state;

    let route = Memo::new(move |_| {
        let hash_str = hash.get();

        let (path, hash_args) = if let Some(captures) = ROUTE_REGEX.captures(&hash_str) {
            let path = captures.name("path").map_or("", |m| m.as_str()).to_owned();
            let args = captures.name("args").map_or("", |m| m.as_str()).to_owned();
            (path, args)
        } else {
            log!("invalid route: {}", hash_str);
            ("".to_owned(), "".to_owned())
        };

        // we want to support anonymous navigation, so we also try to parse the current args from the history state
        // hash args take priority over history state, but we always wanna .get() here tho to ensure proper reactivity
        let state_str = state.get().to_js_value().as_string().unwrap_or_default();
        let args = if !hash_args.is_empty() {
            hash_args
        } else {
            state_str
        };

        let route = AppRoute::parse(&path, args);
        log!("route: {:?}", route);
        route
    });

    move || match route.get() {
        AppRoute::Home => view! { <HomePage /> }.into_any(),
        AppRoute::Trace(args) => view! { <TracePage args /> }.into_any(),
        // TODO: replace these stubs
        AppRoute::Login => view! { "login" }.into_any(),
        AppRoute::NewPost => view! { "new post" }.into_any(),
        AppRoute::NotFound => view! { "not found" }.into_any(),
        AppRoute::Account => view! { "account" }.into_any(),
    }
}
