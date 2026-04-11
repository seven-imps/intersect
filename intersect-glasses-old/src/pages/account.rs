use leptos::*;

use crate::{
    components::{IfSome, LoginForm, LogoutButton, Modal, Selectable},
    session::Session,
};

#[component]
pub fn Account() -> impl IntoView {
    let session = use_context::<RwSignal<Session>>().expect("session context");
    let (session_identity, _) = slice!(session.blog_identity);

    let show_password = create_rw_signal(false);
    let on_click = move |_| {
        show_password.set(true);
    };

    view! {
        <IfSome
            signal = session_identity
            view = move |identity| view! {
                <Selectable text=identity.shard().to_string() label="logged in as"/>
                <div class="account-private-key">
                    <Modal title="private key" show=show_password>
                        // <p> "store this somewhere safe and never share it with anyone!" </p>
                        <Selectable text=identity.private_key().to_string() label="private key"/>
                    </Modal>
                    <button class="links-add-button" type="button" on:click=on_click> "show private key" </button>
                </div>
            }
        / >
        <LoginForm />
        <LogoutButton />
    }
}
