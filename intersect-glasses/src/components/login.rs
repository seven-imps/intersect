use anyhow::{anyhow, Context};
use intersect_core::{models::{IndexMetadata, Segment}, Identity, PrivateKey, RootDomain, Shard};
use leptos::*;
use leptos_router::State;

use crate::{
    components::{Form, IfSome, Modal, Selectable},
    make_action,
    session::{cookie_session_shard, Session},
};

use super::TextInput;

#[component]
pub fn LogoutButton() -> impl IntoView {
    let session = use_context::<RwSignal<Session>>().expect("session context");

    view! {
        // only show if logged in
        <Show
            when= move || session.with(|s| s.is_logged_in())
        >
            <button type="button" on:click=move |_| {
                session.update(|s| s.logout());
            } >"log out"</button>
        </Show>
    }
}

#[component]
pub fn LoginForm() -> impl IntoView {
    let shard_field = create_rw_signal("".to_string());
    let sk_field = create_rw_signal("".to_string());

    let session = use_context::<RwSignal<Session>>().expect("session context");
    let (session_identity, _) = slice!(session.blog_identity);

    // new account logic
    let is_new_account = create_rw_signal(false);
    let show_new_account_modal = create_rw_signal(false);

    let generate_identity = move |_| {
        let identity = Identity::random();
        shard_field.set(identity.shard().to_string());
        sk_field.set(identity.private_key().to_string());

        is_new_account.set(true);
    };

    // login form

    let validate = move |_| {
        if shard_field.get_untracked().is_empty() {
            return Err(anyhow!("shard can't be empty"));
        }
        let shard = Shard::try_from(shard_field.get_untracked().as_str())
            .with_context(|| "invalid shard")?;

        if sk_field.get_untracked().is_empty() {
            return Err(anyhow!("private key can't be empty"));
        }
        let private_key = PrivateKey::try_from(sk_field.get_untracked().as_str())
            .with_context(|| "invalid private key")?;

        let identity =
            Identity::new(shard, private_key).with_context(|| "keypair doesn't match")?;
        return Ok(identity);
    };

    let create_account_root = make_action!(move |identity: &Identity| {
        let root_name = Segment::new("account").unwrap();
        let account_name = Segment::new("anonymous").unwrap();
        let meta = IndexMetadata::new(identity.shard(), &account_name);
        let _index = RootDomain::create_public(&identity, &root_name, &meta)
            .await
            .expect("failed to create account root index");
    });

    let on_login = move |identity: Identity| {
        // clear fields
        shard_field.set("".to_string());
        sk_field.set("".to_string());
        // update our login session
        session.update(|s| s.login(identity));

        if is_new_account.get() {
            show_new_account_modal.set(true);
            create_account_root.dispatch(identity)
        }

        // push a new history entry with a unique state
        // this is to make sure we trigger password managers
        // to prompt to save the keypair
        // and we don't actually want to redirect, but this works
        let navigate = leptos_router::use_navigate();
        let state = shard_field.get_untracked();
        navigate(
            "#/account",
            leptos_router::NavigateOptions {
                state: State(Some(state.into())),
                ..Default::default()
            },
        );
    };

    view! {
        // hide if already logged in
        <Show
            when= move || session.with(|s| !s.is_logged_in())
        >
            <Form validate=validate on_submit=on_login pwd_manager_workarounds=true>
                // the reactive_events is also a password manager workaround
                <TextInput value=shard_field id="username" label="shard: " autocomplete="username" reactive_events=true/>
                <TextInput value=sk_field id="password" label="private key: " input_type="password" autocomplete="current-password" reactive_events=true/>

                <button type="submit">"login"</button>
            </Form>
            <p class="login-or-new-divider"> "or" </p>
            <button type="button" on:click=generate_identity >"create new"</button>
        </Show>

        <Modal title="new account created!" show=show_new_account_modal>
            <p> "before continuing, make sure you store your account details somewhere safe and private!" </p>
            <p> "you will need these next time you log in:" </p>

            <IfSome
                signal = session_identity
                view = move |identity| view! {
                    <Selectable text=identity.shard().to_string() label="shard"/>
                }
            / >
            <IfSome
                signal = session_identity
                view = move |identity| view! {
                    <Selectable text=identity.private_key().to_string() label="private key"/>
                }
            / >
        </Modal>
    }
}

/// component to prompt the user to re-authenticate
#[component]
pub fn PromptReAuth() -> impl IntoView {
    let session = use_context::<RwSignal<Session>>().expect("session context");

    let cookie_shard = cookie_session_shard();
    let show_reauthentication =
        create_memo(move |_| cookie_shard.get().is_some() && !session.with(|s| s.is_logged_in()));

    let sk_field = create_rw_signal("".to_string());

    let validate = move |_| {
        if sk_field.get_untracked().is_empty() {
            return Err(anyhow!("private key can't be empty"));
        }
        let private_key = PrivateKey::try_from(sk_field.get_untracked().as_str())
            .with_context(|| "invalid private key")?;

        let shard = cookie_shard
            .get()
            .ok_or(anyhow!("missing shard in cookie"))?;

        let identity =
            Identity::new(shard, private_key).with_context(|| "keypair doesn't match")?;
        return Ok(identity);
    };

    let on_login = move |identity: Identity| {
        // clear fields
        sk_field.set("".to_string());
        // update our login session
        session.update(|s| s.login(identity));
    };

    let forget_session = move || {
        // clear session cookie
        session.update(|s| s.forget_saved());
    };

    view! {
        <Modal title="re-authenticate" show=show_reauthentication>
            <p> "please enter your private key to log in again as:" </p>
            <IfSome
                signal = cookie_shard
                view = move |shard| view! {
                    <Selectable text=shard.to_string() label="shard"/>
                }
            / >
            <Form validate=validate on_submit=on_login pwd_manager_workarounds=true>
                <TextInput value=sk_field id="password" label="private key: " input_type="password" autocomplete="current-password"/>
                <button type="submit">"login"</button>
            </Form>
            <button type="button" on:click=move |_| forget_session()>"forget"</button>
        </Modal>
    }
}
