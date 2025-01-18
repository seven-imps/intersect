use anyhow::{anyhow, Context};

use intersect_core::models::{Fragment, IndexMetadata, Segment, UnlockedTrace};
use intersect_core::{ContentDomain, Identity, IndexDomain, IndexRecord, RecordType};
use leptos::*;

use crate::components::{Form, IfSome, StatusContext, TextArea, TextInput};
use crate::make_action;
use crate::pages::navigate_to_view;

#[derive(Clone)]
pub struct NewPost {
    segment: Segment,
    text: String,
}

#[component]
pub fn EditorBase(
    #[prop(into)] on_submit: Action<NewPost, ()>,
    #[prop(into, optional)] initial_path: String,
    #[prop(into, optional)] initial_text: String,
) -> impl IntoView {
    let path_field = create_rw_signal(initial_path.to_string());
    let text_field = create_rw_signal(initial_text.to_string());

    // form validation logic
    let validate = move |_| {
        if path_field.get_untracked().is_empty() {
            return Err(anyhow!("name can't be empty"));
        }
        let segment = Segment::new(path_field.get_untracked())?;
        let text = text_field.get_untracked();
        return Ok(NewPost { segment, text });
    };

    // form submission logic
    let on_post = move |new_post: NewPost| {
        on_submit.dispatch(new_post);
    };

    view! {
        <Form validate=validate on_submit=on_post>
            <TextInput value=path_field id="path" label="document name: " />
            <TextArea value=text_field id="text" label="text: " />
            <button>"save"</button>
        </Form>
    }
}

#[component]
pub fn EditorNew(
    #[prop(into)] identity: Signal<Identity>,
    #[prop(into, optional)] trace_out: Option<WriteSignal<Option<UnlockedTrace<IndexRecord>>>>,
    #[prop(into, optional)] initial_path: String,
    #[prop(into, optional)] initial_text: String,
) -> impl IntoView {
    let status_context = expect_context::<StatusContext>();
    let link_signal: RwSignal<Option<UnlockedTrace<IndexRecord>>> = create_rw_signal(None);

    let post = move |new_post: NewPost| async move {
        let identity = identity.get_untracked();
        let fragment = Fragment::new(new_post.text.as_bytes().to_vec());
        let fragment_record = ContentDomain::create(&identity, &fragment).await?;
        let meta = IndexMetadata::new(identity.shard(), &new_post.segment)
            .with_fragment(&fragment_record.to_trace(true));
        let index = IndexDomain::create(&identity, &meta).await?;

        let trace = index.to_unlocked_trace();
        link_signal.set(Some(trace.clone()));
        if let Some(link_out) = trace_out {
            link_out.set(Some(trace.clone()));
        }

        Ok(())
    };

    let post_action = make_action!(move |new_post: &NewPost| {
        let result = status_context
            .run_async(|| post(new_post), Some("uploading..."))
            .await;

        if result.is_ok() {
            // and now redirect to it
            // TODO: clean this up. should be fine but it's janky
            navigate_to_view(
                &link_signal
                    .get_untracked()
                    .expect("link should be present after posting")
                    .into(),
            );
        }
    });

    view! {
        <EditorBase on_submit=post_action initial_path=initial_path initial_text=initial_text />
    }
}

// TODO: finish
#[component]
pub fn EditorExisting(
    #[prop(into)] identity: Identity,
    #[prop(into)] trace: UnlockedTrace<IndexRecord>,
) -> impl IntoView {
    let status_context = expect_context::<StatusContext>();

    let metadata_signal: RwSignal<Option<IndexMetadata>> = create_rw_signal(None);
    let initial_text_signal: RwSignal<String> = create_rw_signal("".to_owned());
    let initial_path_signal: RwSignal<String> = create_rw_signal("".to_owned());

    let load_document = move || async move {
        // load index
        let index_record = trace.open().await.with_context(|| "couldn't open trace")?;

        let meta = index_record.meta(true).await?;
        metadata_signal.set(Some(meta.clone()));
        initial_path_signal.set(meta.name().to_string());

        // load fragment
        if let Some(fragment_trace) = meta.fragment() {
            let fragment_record = fragment_trace
                .try_open()
                .await
                .with_context(|| "error while fetching fragment")?;

            let fragment = fragment_record
                .load()
                .await
                .with_context(|| "error while fetching fragment")?;

            let raw_text = String::from_utf8_lossy(&fragment.data).to_string();
            // save initial text
            initial_text_signal.set(raw_text);
        }
        Ok(())
    };

    let load_document = make_action!(move |_| {
        let _ = status_context
            .run_async(load_document, Some("refreshing document..."))
            .await;
    });

    load_document.dispatch(());

    let post = move |new_post: NewPost| async move {
        // load index record
        let mut index_record = trace.open().await.with_context(|| "couldn't open trace")?;

        // build fragment
        let fragment = Fragment::new(new_post.text.as_bytes().to_vec());
        // and upload it
        let fragment_record = ContentDomain::create(&identity, &fragment).await?;

        // grab the most recent index metadata
        let meta = index_record.meta(true).await?;
        // ... update it
        let updated_meta = meta
            .with_fragment(&fragment_record.to_trace(true))
            .with_name(&new_post.segment);

        // ... and save it to the record
        index_record.update_meta(&identity, &updated_meta).await?;

        Ok(())
    };

    let post_action = make_action!(move |new_post: &NewPost| {
        let result = status_context
            .run_async(|| post(new_post), Some("uploading..."))
            .await;

        if result.is_ok() {
            // and now redirect to it
            navigate_to_view(&trace.into())
        }
    });

    view! {
        <IfSome
            signal = load_document.value()
            // fallback = || view! { <p> "loading document..." </p> }
            view = move |_| view! {
                <EditorBase on_submit=post_action initial_path=initial_path_signal.get() initial_text=initial_text_signal.get() />
            }
        / >
    }
}
