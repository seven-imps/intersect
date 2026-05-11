use components::{PromptReAuth, Status, StatusContext};
use intersect_core::log;
use lazy_regex::{lazy_regex, Lazy, Regex};
use leptos::*;
use leptos_router::*;
use pages::{home::Home, Account, Edit, NotFound, Post, Trace, View};

pub mod components;
pub mod intersect;
pub mod leptos_helpers;
pub mod pages;
pub mod session;
use intersect::Intersect;

pub static HASHROUTE_REGEX: Lazy<Regex> = lazy_regex!(r"#/(?<path>[^/]+)(/(?<args>.*))?$");

#[component]
pub fn HashRouter() -> impl IntoView {
    log!("router initialised");
    let hash_memo = use_location().hash;
    let state_signal = use_location().state;

    // TODO: the semantics here are wrong.
    // really, the output of the memo signal should be an enum of
    // all the pages and potentially their associated inputs
    // *that's* the thing we want care about for reactivity updates
    let location_signal = create_memo(move |_| {
        // the entire hash
        let hash_str = hash_memo.get();
        // use our regex to validate and parse it
        let (path, args) = if let Some(captures) = HASHROUTE_REGEX.captures(&hash_str) {
            // the page name
            let path = captures
                .name("path")
                .map_or("".to_owned(), |m| m.as_str().to_string());
            // and everything that comes after
            let args = captures
                .name("args")
                .map_or("".to_owned(), |m| m.as_str().to_string());

            // return as tuple
            (path, args)
        } else {
            // empty by default
            ("".to_owned(), "".to_owned())
        };

        // grab state
        let state: String = state_signal
            .get()
            .to_js_value()
            .as_string()
            .unwrap_or("".to_owned());
        // .expect("history state should always be a string");

        log!("route evaluated: {path} / {args} / {}", !state.is_empty());
        (path, args, state)
    });

    let router_view = move || {
        log!("router output rendered");
        let (path, args, state) = location_signal.get();

        // reset status when we navigate
        let status_context = expect_context::<StatusContext>();
        status_context.clear();

        let page_view = match path.as_str() {
            // TODO: gotta find a better way to wrap these, but whatever :p
            "" | "home" => view! {<Home />},
            "trace" => view! {<Trace hash_args=args.clone()/>},
            "view" => view! {<View state=state.clone()/>},
            "edit" => view! {<Edit state=state.clone()/>},
            "post" => view! {<Post />},
            "account" => view! {<Account />},
            _ => view! {<NotFound/>},
        };

        page_view
    };

    // core shell
    view! {
        <Status>
            // don't mess this up!!
            // the intersect component should only ever run once!
            <Intersect>
                <PromptReAuth />
                { router_view }
            </Intersect>
        </Status>
    }
}

fn main() {
    // set up logging
    // _ = console_log::init_with_level(log::Level::Debug);
    intersect_core::setup_wasm_logging();
    console_error_panic_hook::set_once();

    mount_to_body(|| {
        view! {
            <Router>
                <Routes>
                    <Route path="" view=HashRouter/>
                </Routes>
            </Router>
        }
    })
}
