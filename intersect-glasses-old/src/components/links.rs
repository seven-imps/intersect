use anyhow::Ok;
use intersect_core::{
    log,
    models::{LinkEntry, UnlockedTrace}, LinksRecord, ValueSubkey,
};
use leptos::*;

use crate::{
    components::{Collapsible, IntersectLink}, make_action, session::Session
};

use super::{IfSome, StatusContext};

#[component]
pub fn Links(
    trace: UnlockedTrace<LinksRecord>,
    #[prop(into)] is_editable: Signal<bool>,
) -> impl IntoView {
    let status_context = expect_context::<StatusContext>();

    let links_signal: RwSignal<Option<Vec<(ValueSubkey, LinkEntry)>>> = create_rw_signal(None);

    let session = expect_context::<RwSignal<Session>>();
    let identity = session.with_untracked(|s| s.blog_identity);
    let show_edit_menu = move || 
        is_editable.get() 
        // && session.get().is_authorised(&trace.into())
    ;

    let fetch_links = move |trace: UnlockedTrace<LinksRecord>, force_refresh| async move {
        if force_refresh {
            log!("loading index links from network");
        } else {
            log!("loading index links from local store");
        }
        let record = trace.open().await?;
        let links = record.fetch_links(force_refresh).await?;
        if Some(links.clone()) != links_signal.get_untracked() {
            links_signal.set(Some(links))
        }
        // log!("links found: {:?}", links);
        Ok(())
    };

    let remove_link = move |trace: UnlockedTrace<LinksRecord>, identity, subkey| async move {
        let record = trace.open().await?;
        record.remove_link(&identity, subkey).await?;
        Ok(())
    };

    let remove_link_action = make_action!(move |subkey: &ValueSubkey| {
        log!("removing link");
        let _ = status_context.run_async(|| remove_link(
            trace,
            identity.expect("identity").clone(),
            subkey.clone(),
        ), Some("removing link...")).await;

        // and refresh
        let _ = status_context.run_async(|| fetch_links(trace, false), None).await;
    });

    // kick off the download
    spawn_local(async move {
        // initial lazy load
        let _ = status_context.run_async(|| fetch_links(trace, false), None).await;
        // and hard refresh
        let _ = status_context.run_async(|| fetch_links(trace, true), None).await;
    });

    let render_link = move |subkey: ValueSubkey, index_link: &LinkEntry| {
        let name = index_link.name().to_string();
        // match  {
        //     Link::Open(reference) => {

        //     }
        //     None => view! { <p class="links-link"> {name} "[encrypted]" </p> }.into_view(),
        // };
        view! {
            <li>
                <div class="links-li">
                    <IntersectLink trace=index_link.trace().clone() text=name class="links-link"/>
                    <Show
                        when=show_edit_menu
                    >
                        <button class="links-remove" type="button" on:click=move |_| {
                            remove_link_action.dispatch(subkey)
                        } >"×"</button>
                    </Show>
                </div>
            </li>
        }
    };

    let should_show = move || {
        let has_links = links_signal.get().is_some_and(|l| l.len() > 0);
        // has_links || show_edit_menu()
        has_links
    };

    view! {
        <Show
            when=should_show
        >
            <div class="links">
                <Collapsible summary=move || "links" default_open=true >
                    <IfSome
                        signal = links_signal
                        fallback = || view! { <p> "loading links..." </p> }
                        view = move |links| view! {
                            <ul class="links-list" role="list">
                            {   log!("links rendered");
                                links
                                .into_iter()
                                .map(|link| render_link(link.0, &link.1))
                                .collect_view()
                            }
                            </ul>
                        }
                    / >
                </Collapsible>
            </div>
        </Show>
    }
}
