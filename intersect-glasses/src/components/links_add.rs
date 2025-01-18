use anyhow::{Context, Ok};
use intersect_core::{
    models::{LinkEntry, Segment, Trace, UnlockedTrace},
    IndexRecord, IntersectError, LinksDomain, RecordType,
};
use leptos::*;

use crate::{
    components::{empty_view, TextInput},
    make_action,
    session::Session,
};

use super::{Form, Modal, StatusContext};

#[derive(Clone)]
struct NewLink {
    index_reference: UnlockedTrace<IndexRecord>,
    sublink_name: Segment,
    sublink_trace: Trace<IndexRecord>,
}

#[component]
pub fn LinksAddModal(
    trace: UnlockedTrace<IndexRecord>,
    on_update: Action<(), ()>,
    show: RwSignal<bool>,
) -> impl IntoView {
    let status_context = expect_context::<StatusContext>();

    // grab identity
    let session = expect_context::<RwSignal<Session>>();
    let identity = session.get_untracked().blog_identity;

    // unpack identity and ensure we're logged in
    let Some(identity) = identity else {
        return empty_view();
    };

    // signals for the inputs
    let sublink_name = create_rw_signal("".to_string());
    let sublink_trace = create_rw_signal("".to_string());

    // the actual link adding logic
    let add_link = move |new_link: NewLink| async move {
        // open the index
        let mut index = new_link
            .index_reference
            .open()
            .await
            .with_context(|| "error while initialising record")?;

        // and validate that our identity matches the trace
        // (though even if we didn't check it'd still fail to write to the record anyway)
        if identity.shard() != index.record().shard() {
            Err(IntersectError::Unauthorized)?;
        }

        // let links = index.fetch_or_new_links(&identity, true).await?;

        let links = match index.try_fetch_links(true).await? {
            Some(record) => record,
            None => {
                // create empty links record
                let record = LinksDomain::create(&identity, &[]).await?;
                // and save it to the metadata
                let new_meta = index
                    // no need to refresh meta, fetch_links already did
                    .meta(false)
                    .await?
                    .with_links(&record.to_trace(true));
                index.update_meta(&identity, &new_meta).await?;

                record
            }
        };

        // and add the link
        let link = LinkEntry::new(&new_link.sublink_name, &new_link.sublink_trace);
        links.add_link(&identity, &link).await?;

        // and call the refresh callback
        on_update.dispatch(());

        Ok(())
    };

    // wrap that logic in an action
    let add_link_action = make_action!(move |new_link: &NewLink| {
        let _ = status_context
            .run_async(|| add_link(new_link), Some("adding link..."))
            .await;
    });

    // form validation logic
    let validate = move |_| {
        // parse inputs
        let sublink_name = Segment::new(sublink_name.get_untracked())?;
        let sublink_trace = Trace::from_str(&sublink_trace.get_untracked())?;

        return Ok(NewLink {
            index_reference: trace.clone(),
            sublink_name,
            sublink_trace,
        });
    };

    // form submission logic
    let on_submit = move |new_link| {
        add_link_action.dispatch(new_link);
        show.set(false);
    };

    view! {
        <Modal title="add new link" show=show>
            <Form validate=validate.clone() on_submit=on_submit>
                <TextInput value=sublink_name id="links-add-name" label="name: " />
                <TextInput value=sublink_trace id="links-add-trace" label="trace: " />
                <button>"confirm"</button>
            </Form>
        </Modal>
    }
    .into_view()
}

#[component]
pub fn LinksAddButton(
    trace: UnlockedTrace<IndexRecord>,
    on_update: Action<(), ()>,
) -> impl IntoView {
    let show_modal = create_rw_signal(false);

    view! {
        <div class="links-add">
            // button to show the modal
            <button class="links-add-button" type="button" on:click=move |_| {
                show_modal.set(true);
            }>"add link"</button>
            // and the modal/form
            <LinksAddModal trace=trace on_update=on_update show=show_modal />
        </div>
    }
    .into_view()
}
