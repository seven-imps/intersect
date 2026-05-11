use leptos::prelude::*;

use crate::{
    components::{AccountDisplay, Login},
    shell::{use_account, use_intersect},
};

#[component]
pub fn AccountPage() -> impl IntoView {
    let account = use_account();

    move || {
        if let Some(account_ref) = account.get() {
            let intersect = use_intersect();
            view! {
                <AccountDisplay account_ref />
                <button type="button" on:click=move |_| intersect.logout()>
                    "log out"
                </button>
            }
            .into_any()
        } else {
            view! { <Login /> }.into_any()
        }
    }
}
