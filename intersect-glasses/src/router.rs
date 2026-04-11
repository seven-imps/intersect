use intersect_core::log;
use lazy_regex::{Lazy, Regex, lazy_regex};
use leptos::prelude::*;
use leptos_router::hooks::use_location;

// matches #/<path> or #/<path>?<args>
pub static ROUTE_REGEX: Lazy<Regex> = lazy_regex!(r"#/(?<path>[^?]*)(\?(?<args>.*))?$");

/// all app routes with their associated arguments
/// (currently only a single string arg is supported)
#[derive(Clone, PartialEq, Debug)]
pub enum AppRoute {
    Home,
    Trace(String),
    Login,
    NotFound,
}

fn parse_route(path: &str, args: String) -> AppRoute {
    match path {
        "" | "home" => AppRoute::Home,
        "trace" => AppRoute::Trace(args),
        "login" => AppRoute::Login,
        _ => AppRoute::NotFound,
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

        let route = parse_route(&path, args);
        log!("route: {:?}", route);
        route
    });

    move || match route.get() {
        // TODO: replace with actual views once we have them
        AppRoute::Home => view! { "home" }.into_any(),
        AppRoute::Trace(args) => view! { "trace: " {args} }.into_any(),
        AppRoute::Login => view! { "login" }.into_any(),
        AppRoute::NotFound => view! { "not found" }.into_any(),
    }
}
