use intersect_core::{TypedReference, documents::AccountDocument};
use leptos::prelude::*;

use crate::components::{NetworkSuspend, use_open};

#[component]
pub fn AccountDisplay(account_ref: TypedReference<AccountDocument>) -> impl IntoView {
    let signal = use_open(account_ref);

    view! {
        <NetworkSuspend signal let:account>
            {
                let fingerprint = account.public_key().fingerprint();
                let name = account.name().map(|n| n.as_ref().to_owned()).unwrap_or("anonymous".into());
                let bio = account.bio().map(|b| b.as_ref().to_owned());
                view! {
                    <div class="account-view">

                        <p class="account-name">
                            {name}
                            <span class="account-fingerprint">"#" {fingerprint}</span>
                        </p>
                        {bio.map(|b| view! { <p class="account-bio">{b}</p> })}
                    </div>
                }
            }
        </NetworkSuspend>
    }
}
