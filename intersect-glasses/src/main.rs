use leptos::prelude::*;
use leptos_router::components::Router;

mod components;
mod pages;
mod router;
mod shell;
mod util;

use router::HashRouter;
use shell::Shell;

fn main() {
    console_error_panic_hook::set_once();

    mount_to_body(|| {
        view! {
            // leptos router without routes cause we use our own hash router inside
            // this just makes sure the reactive location hooks are all set up
            <Router>
                // intersect shell inside the router, only loaded once
                <Shell>
                    // and then our internal router to render all the dynamic pages
                    <HashRouter/>
                </Shell>
            </Router>
        }
    })
}
