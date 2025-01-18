use comrak::markdown_to_html;
use intersect_core::{
    models::{Fragment, UnlockedTrace},
    FragmentRecord,
};
use leptos::*;

use crate::components::{IfSome, StatusContext};

#[component]
pub fn FragmentView(trace: UnlockedTrace<FragmentRecord>) -> impl IntoView {
    let status_context = expect_context::<StatusContext>();

    let fragment_signal: RwSignal<Option<Fragment>> = create_rw_signal(None);

    let fetch_fragment = move || async move {
        let record = trace.open().await?;
        let fragment = record.load().await?;
        fragment_signal.set(Some(fragment));
        Ok(())
    };

    spawn_local(async move {
        let _ = status_context.run_async(fetch_fragment, None).await;
    });

    let fragment_view = move |fragment: Fragment| {
        let mut markdown_options = comrak::Options::default();
        markdown_options.render.escape = true;
        // markdown_options.extension.header_ids = Some("document-content-".to_string());
        // markdown_options.extension.footnotes = true;
        markdown_options.extension.multiline_block_quotes = true;
        markdown_options.extension.math_code = true;
        markdown_options.extension.shortcodes = true;

        let raw_text = String::from_utf8_lossy(&fragment.data).to_string();
        let html = markdown_to_html(&raw_text, &markdown_options);

        view! {
            <main inner_html=html/>
        }
        .into_view()
    };

    view! {
        <IfSome
            signal = fragment_signal
            fallback = || view! { <p> "downloading..." </p> }
            view = fragment_view
        / >
    }
}
