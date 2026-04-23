use std::str::FromStr;

use anyhow::anyhow;
use intersect_core::{
    TypedReference,
    documents::AccountDocument,
    models::{AccountSecret, Trace},
};
use leptos::prelude::*;
use leptos::task::spawn_local;

use leptos_router::hooks::use_navigate;

use crate::{
    components::{
        base::{Form, TextArea, TextInput},
        use_loading,
    },
    router::{AppRoute, navigate_to},
    shell::use_intersect,
};

#[derive(Clone, PartialEq)]
enum LoginMode {
    Login,
    Create,
}

#[component]
pub fn Login() -> impl IntoView {
    let mode: RwSignal<LoginMode> = RwSignal::new(LoginMode::Login);

    let toggle = move |_| {
        mode.update(|m| {
            *m = match m {
                LoginMode::Login => LoginMode::Create,
                LoginMode::Create => LoginMode::Login,
            }
        })
    };

    view! {
        {move || match mode.get() {
            LoginMode::Login => view! { <LoginForm /> }.into_any(),
            LoginMode::Create => view! { <CreateForm /> }.into_any(),
        }}
        <p class="login-or-new-divider">"or"</p>
        <button type="button" on:click=toggle>
            {move || match mode.get() {
                LoginMode::Login => "create new account",
                LoginMode::Create => "back to log in",
            }}
        </button>
    }
}

struct LoginFormData {
    account_ref: TypedReference<AccountDocument>,
    secret: AccountSecret,
}

#[component]
fn LoginForm() -> impl IntoView {
    let trace_input = RwSignal::new(String::new());
    let secret_input = RwSignal::new(String::new());

    let validate = Callback::new(move |()| -> Result<LoginFormData, anyhow::Error> {
        let account_ref = Trace::from_str(&trace_input.get_untracked())
            .map_err(|_| anyhow!("invalid trace"))?
            .into_typed::<AccountDocument>()
            .map_err(|_| anyhow!("trace is not an account"))?
            .into_unlocked()
            .map_err(|_| anyhow!("account trace must be unlocked"))?;

        let secret = AccountSecret::from_str(&secret_input.get_untracked())
            .map_err(|_| anyhow!("invalid secret"))?;

        Ok(LoginFormData {
            account_ref,
            secret,
        })
    });

    let intersect = use_intersect();
    let loading = use_loading();
    let navigate = use_navigate();

    let on_submit = Callback::new(move |data: LoginFormData| {
        let intersect = intersect.clone();
        let loading = loading.clone();
        let navigate = navigate.clone();

        spawn_local(async move {
            let result = loading
                .run(
                    || async move {
                        intersect
                            .login(data.account_ref, data.secret)
                            .await
                            .map_err(|e| anyhow!(e))
                    },
                    "logging in...",
                )
                .await;
            // errors are surfaced as an overlay by loading.run, no need to handle here
            if result.is_ok() {
                // push a history entry on submit so password managers see a "navigation" and prompt to save
                navigate_to(&navigate, AppRoute::Account);
            }
        });
    });

    view! {
        <Form validate on_submit pwd_manager_workarounds=true>
            <TextInput
                value=trace_input
                id="login-trace"
                label="account"
                autocomplete="username"
                reactive_events=true
            />
            <TextInput
                value=secret_input
                id="login-secret"
                label="secret key"
                input_type="password"
                autocomplete="current-password"
                reactive_events=true
            />
            <button type="submit">"log in"</button>
        </Form>
    }
}

struct CreateFormData {
    name: Option<String>,
    bio: Option<String>,
}

#[component]
fn CreateForm() -> impl IntoView {
    let name_input = RwSignal::new(String::new());
    let bio_input = RwSignal::new(String::new());

    let validate = Callback::new(move |()| -> Result<CreateFormData, anyhow::Error> {
        let name = name_input.get_untracked();
        let bio = bio_input.get_untracked();
        Ok(CreateFormData {
            name: (!name.is_empty()).then_some(name),
            bio: (!bio.is_empty()).then_some(bio),
        })
    });

    let intersect = use_intersect();
    let loading = use_loading();
    let navigate = use_navigate();

    let on_submit = Callback::new(move |data: CreateFormData| {
        let intersect = intersect.clone();
        let loading = loading.clone();
        let navigate = navigate.clone();

        spawn_local(async move {
            let result = loading
                .run(
                    || async move {
                        intersect
                            .create_account(data.name, data.bio, None)
                            .await
                            .map_err(|e| anyhow!(e))
                    },
                    "creating account...",
                )
                .await;
            // errors are surfaced as an overlay by loading.run, no need to handle here
            if result.is_ok() {
                // push a history entry on submit so password managers see a "navigation" and prompt to save
                navigate_to(&navigate, AppRoute::Account);
            }
        });
    });

    view! {
        <Form validate on_submit pwd_manager_workarounds=true>
            <TextInput value=name_input id="create-name" label="name" />
            <TextArea value=bio_input id="create-bio" label="bio" />
            <button type="submit">"create account"</button>
        </Form>
    }
}
