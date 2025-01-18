use anyhow::{Context, Ok};
use intersect_core::{
    models::{IndexMetadata, UnlockedTrace},
    IndexRecord,
};
use leptos::*;
use phosphor_leptos::{Icon, IconWeight};

use crate::{
    components::{
        empty_view, fragment::FragmentView, ActionLink, IfSome, IntersectEditLink, Links,
        LinksAddModal, Modal, ShareTrace, StatusContext,
    },
    make_action,
    session::Session,
};

#[component]
pub fn Document(
    trace: UnlockedTrace<IndexRecord>,
    #[prop(optional)] hide_header: bool,
) -> impl IntoView {
    let status_context = expect_context::<StatusContext>();
    let session = expect_context::<RwSignal<Session>>();

    let metadata_signal: RwSignal<Option<IndexMetadata>> = create_rw_signal(None);
    let is_owner = move || {
        metadata_signal
            .get()
            .is_some_and(|meta| session.get().is_authorised(meta.shard()))
    };

    let fetch_record = move |force_refresh: bool| async move {
        let record = trace.open().await.with_context(|| "couldn't open trace")?;

        let meta = record.meta(force_refresh).await?;
        metadata_signal.set(Some(meta.clone()));

        // read account root
        // let account_root =
        //     RootDomain::open_public(meta.shard(), &Segment::new("account").unwrap()).await;
        // if let Result::Ok(account_root) = account_root {
        //     let account_meta = account_root.meta(force_refresh).await?;
        //     log!("account username: {}", account_meta.name());
        // }

        Ok(meta)
    };

    // kick off the download
    spawn_local(async move {
        // initial lazy load
        let _ = status_context
            .run_async(|| fetch_record(false), Some("loading index..."))
            .await;
        // and hard refresh
        let _ = status_context.run_async(|| fetch_record(true), None).await;
    });

    let fragment_signal = create_read_slice(metadata_signal, |meta| {
        meta.as_ref().and_then(|meta| meta.fragment().cloned())
    });

    let links_signal = create_read_slice(metadata_signal, |meta| {
        meta.as_ref().and_then(|meta| meta.links().cloned())
    });

    // this is a bit hacky
    // but this is so we can force a links refresh when we add a new link
    let should_refresh_links: RwSignal<bool> = create_rw_signal(false);
    let on_links_update = make_action!(move |_| {
        let _ = status_context
            .run_async(|| fetch_record(false), None)
            .await
            .and_then(|_| Ok(should_refresh_links.set(true)));
    });

    let show_share = create_rw_signal(false);
    let show_menu_dropdown = create_rw_signal(false);
    let show_link_edit = create_rw_signal(false);
    let show_link_add = create_rw_signal(false);

    let header = move || {
        view! {
        <header>
            <div class="document-details">
                <div class="document-details-left">
                    // <IntersectLink link=todo!() text="username"/>
                    // <p> " — " </p>
                    // <p class="document-timestamp"> "2024/07/23" </p>
                    <p class="document-timestamp"> "placeholder" </p>
                </div>

                <div class="document-details-right">
                    // share button
                    <button title="share" class="document-share button-icon" type="button" on:click=move |_| show_share.set(true)>
                        <Icon icon=phosphor_leptos::LINK weight=IconWeight::Bold />
                    </button>
                    <Modal title="share" show=show_share>
                        <ShareTrace trace=trace />
                    </Modal>

                    // dropdown menu
                    <Show
                        when=is_owner
                    >
                        <button
                            title="document menu" class="document-menu button-icon" type="button"
                            id="document-menu-button"
                            aria-controls="document-menu-elements"
                            aria-expanded= move || if show_menu_dropdown.get() { "true" } else { "false" }
                            on:click= move |_| show_menu_dropdown.update(|s| *s = !(*s))
                        >
                            // <Icon icon=phosphor_leptos::DOTS_THREE_VERTICAL weight=IconWeight::Bold />
                            <Icon icon=phosphor_leptos::DOTS_THREE_OUTLINE_VERTICAL weight=IconWeight::Fill />
                            <ul id="document-menu-elements" aria-labelledby="document-menu-button">
                                <li> <IntersectEditLink trace=trace text="edit note" title="edit note"/> </li>
                                <li>
                                    <ActionLink on_click= move |_| show_link_add.update(|s| *s = true) text="add link"/>
                                    <LinksAddModal trace=trace on_update=on_links_update show=show_link_add />
                                </li>
                                <li> <ActionLink on_click= move |_| show_link_edit.update(|s| *s = !(*s)) text="edit links"/> </li>
                            </ul>
                        </button>
                    </Show>
                </div>
            </div>

            // title
            <IfSome
                signal = metadata_signal
                view = move |meta| view! {
                    <h1 class="document-title">{ meta.name().to_string() }</h1>
                    // <div class="document-divider"></div>
                }
            / >
        </header>
    }.into_view()
    };

    view! {
        <section class="document">
            { if !hide_header { header() } else { empty_view()} }

            // links
            <IfSome
                signal = move || {
                    // just so we also refresh when this gets updated
                    should_refresh_links.get();
                    // but just return the links_signal
                    links_signal.get()
                }
                view = move |links_trace| view! {
                    <Links trace=links_trace.try_into().expect("links trace should be unlocked") is_editable=show_link_edit/>
                }
            / >

            <IfSome
                signal = fragment_signal
                fallback = || view! { <p> "[no fragment]" </p> }
                view = move |fragment_trace| view! {
                    <FragmentView trace=fragment_trace.try_into().expect("fragment trace should be unlocked") />
                }
            / >
        </section>
    }
}
