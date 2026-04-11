use leptos::prelude::*;
use leptos_router::components::Router;

mod router;
mod shell;
mod util;

use router::HashRouter;
use shell::Shell;

fn main() {
    console_error_panic_hook::set_once();

    mount_to_body(|| {
        view! {
            // router without routes cause we use our own hash router inside
            <Router>
                // intersect shell inside the router so it's always there
                // and only loaded once
                <Shell>
                    // and then our internal router to render the pages
                    <HashRouter/>
                </Shell>
            </Router>
        }
    })
}
