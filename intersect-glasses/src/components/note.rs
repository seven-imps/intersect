use comrak::{Options, markdown_to_html};
use intersect_core::documents::FragmentView;
use leptos::prelude::*;

#[component]
pub fn Note(fragment: FragmentView) -> impl IntoView {
    let mut options = Options::default();
    options.render.escape = true;
    options.extension.multiline_block_quotes = true;
    options.extension.math_code = true;
    options.extension.shortcodes = true;
    // options.extension.header_ids = Some("document-content-".to_string());
    // options.extension.footnotes = true;

    let text = String::from_utf8_lossy(fragment.data()).into_owned();
    let html = markdown_to_html(&text, &options);

    // TODO: ponder the potential XSS vuln here more. comrak does do some checking
    // but if we can do additional validation or protections on our end that would be ideal
    // perhaps the play is to have a fully separated iframe or something for the note that
    // doesn't have access to the parent app at all so any potential issues are contained
    view! {
        <article class="note" inner_html=html />
    }
}
