use intersect_core::log;
use lazy_regex::{Lazy, Regex, lazy_regex};
use leptos::prelude::*;
use leptos_router::{
    NavigateOptions,
    hooks::{use_location, use_navigate},
    location::State,
};
use web_sys::UrlSearchParams;
use web_sys::wasm_bindgen::JsValue;

use crate::pages::{AccountPage, HomePage, TracePage};

// matches #/<path> or #/<path>?<args>
pub static ROUTE_REGEX: Lazy<Regex> = lazy_regex!(r"#/(?<path>[^?]*)(\?(?<args>.*))?$");

/// all app routes with their associated arguments
#[derive(Clone, PartialEq, Debug)]
pub enum AppRoute {
    Home,
    Trace { trace: String },
    NewPost,
    Account,
    NotFound,
}

pub struct NavTarget {
    pub url: String,
    pub state: Option<String>,
}

impl NavTarget {
    pub fn new(url: impl Into<String>, params: Vec<(&'static str, String)>) -> Self {
        let state = if params.is_empty() {
            None
        } else {
            let qs = UrlSearchParams::new().unwrap();
            for (key, value) in &params {
                qs.set(key, value);
            }
            Some(qs.to_string().as_string().unwrap_or_default())
        };
        NavTarget {
            url: url.into(),
            state,
        }
    }
}

impl AppRoute {
    /// parse a route from a path and url search params.
    /// `params` must be a valid query string (e.g., `form=emptiness&=emptiness=form`)
    /// strips a leading "#/" if present, so callers can pass either form.
    pub fn parse(path: &str, params: &str) -> Self {
        let params = UrlSearchParams::new_with_str(params)
            .unwrap_or_else(|_| UrlSearchParams::new().unwrap());

        let path = path.strip_prefix("#/").unwrap_or(path);
        match path {
            "" | "home" => AppRoute::Home,
            "trace" => AppRoute::Trace {
                trace: params.get("trace").unwrap_or_default(),
            },
            "new" => AppRoute::NewPost,
            "account" => AppRoute::Account,
            _ => AppRoute::NotFound,
        }
    }

    /// maps route to the base url and optional state payload (as a query-param string).
    /// (use shareable_url() if you need a url with state appended as query params)
    pub fn nav_target(&self) -> NavTarget {
        match self {
            AppRoute::Home => NavTarget::new("#/", vec![]),
            AppRoute::Trace { trace } => NavTarget::new(
                "#/trace",
                vec![("trace", trace.clone())],
            ),
            AppRoute::NewPost => NavTarget::new("#/new", vec![]),
            AppRoute::Account => NavTarget::new("#/account", vec![]),
            AppRoute::NotFound => NavTarget::new("#/nothing", vec![]),
        }
    }

    /// url with state encoded as query params, for sharing / external links.
    /// shouldn't be used for internal navigation!
    pub fn shareable_url(&self) -> String {
        let target = self.nav_target();
        match target.state {
            Some(state) => format!("{}?{}", target.url, state),
            None => target.url,
        }
    }
}

/// navigate to an app route
pub fn navigate_to(navigate: &impl Fn(&str, NavigateOptions), route: AppRoute, replace: bool) {
    let target = route.nav_target();
    navigate(
        &format!("/{}", target.url),
        NavigateOptions {
            replace,
            // state is always passed via history state, never in the url
            state: State::new(target.state.map(|s| JsValue::from_str(&s))),
            ..Default::default()
        },
    );
}

#[component]
pub fn HashRouter() -> impl IntoView {
    log!("router initialised");
    let hash = use_location().hash;
    let state = use_location().state;
    let navigate = use_navigate();

    // parse hash string into base path and query params
    let hash_parts = Memo::new(move |_| {
        let hash_str = hash.get();
        ROUTE_REGEX
            .captures(&hash_str)
            .map(|c| {
                let path = c.name("path").map_or("", |m| m.as_str()).to_owned();
                let args = c.name("args").map_or("", |m| m.as_str()).to_owned();
                (path, args)
            })
            .unwrap_or_else(|| {
                log!("invalid route: {}", hash_str);
                ("".to_owned(), "".to_owned())
            })
    });

    // memo to ensure the route only updates when the route actually meaningfully changes
    let route_memo = Memo::new(move |_| {
        let (path, hash_args) = hash_parts.get();

        // hash args take priority over history state, but we always wanna .get() here tho to ensure proper reactivity
        let state_args = state.get().to_js_value().as_string().unwrap_or_default();

        let route = AppRoute::parse(
            &path,
            if !hash_args.is_empty() {
                &hash_args
            } else {
                &state_args
            },
        );
        log!("route: {:?}", route);
        route
    });

    // migrate inbound query-param urls: move args into history state and clean the url
    Effect::new(move |_| {
        let (_, hash_args) = hash_parts.get();
        if !hash_args.is_empty() {
            log!("moving query params to state");
            navigate_to(&navigate, route_memo.get_untracked(), true);
        }
    });

    move || match route_memo.get() {
        AppRoute::Home => view! { <HomePage /> }.into_any(),
        AppRoute::Trace { trace } => view! { <TracePage trace /> }.into_any(),
        AppRoute::Account => view! { <AccountPage /> }.into_any(),
        // TODO: replace these stubs
        AppRoute::NewPost => view! { "new post" }.into_any(),
        AppRoute::NotFound => view! { "not found" }.into_any(),
    }
}
