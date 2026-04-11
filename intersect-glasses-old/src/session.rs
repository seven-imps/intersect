use codee::string::FromToStringCodec;
use intersect_core::{Identity, Shard};
use leptos::*;
use leptos_use::{use_cookie_with_options, SameSite, UseCookieOptions};

fn use_cookie_session_shard() -> (Signal<Option<Shard>>, WriteSignal<Option<Shard>>) {
    use_cookie_with_options::<Shard, FromToStringCodec>(
        "session-shard",
        UseCookieOptions::default()
            // 7 days (in milliseconds)
            .max_age(7 * 24 * 3600 * 1000)
            .same_site(SameSite::Strict),
    )
}

pub fn cookie_session_shard() -> Signal<Option<Shard>> {
    let (cookie_shard, _) = use_cookie_session_shard();
    cookie_shard
}

#[derive(PartialEq, Clone)]
pub struct Session {
    pub blog_identity: Option<Identity>,
}

impl Session {
    pub fn new() -> Self {
        Session {
            blog_identity: None,
        }
    }

    pub fn login(&mut self, identity: Identity) {
        self.blog_identity = Some(identity);

        // save shard in cookie
        // *not* the private key!
        // this is just so we can easily prompt the user to log in again
        let (_, set_cookie_shard) = use_cookie_session_shard();
        set_cookie_shard.set(Some(*identity.shard()));
    }

    pub fn logout(&mut self) {
        // forget first, so we don't prompt the reauth dialog
        self.forget_saved();
        self.blog_identity = None;
    }

    pub fn forget_saved(&mut self) {
        let (_, set_cookie_shard) = use_cookie_session_shard();
        set_cookie_shard.set(None);
    }

    pub fn is_authorised(&self, shard: &Shard) -> bool {
        self.blog_identity
            .is_some_and(|i| i.shard() == shard)
    }

    pub fn is_logged_in(&self) -> bool {
        self.blog_identity.is_some()
    }
}
