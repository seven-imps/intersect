use leptos::*;

use crate::components::{Collapsible, Lookup};

/// Default Home Page
#[component]
pub fn Home() -> impl IntoView {
    view! {
        <p class="home-tagline">"a decentralized and privacy-oriented tool to store and share text"</p>

        <Lookup />

        <div class="home-about">

            <Collapsible
                // summary=move || view! { <h2 class="home-subtitle"> "what is the intersect?" </h2> }
                summary=move || view! { "what is the intersect?" }
            >
                <p> "the intersect is a protocol, not a website or server. it has no location or owner." </p>

                <p> """
                    data is stored within the
                    """ 
                    <a href="https://veilid.com/"> "veilid" </a>
                    """
                    network, 
                    divided into small encrypted chunks and placed redundantly across many participating devices. 
                    fetching data requires an address, decoding it requires an encryption key. 
                    the address and key together make a trace. 
                    by copying and sharing this trace in your messaging app of choice, you can share access to a post or blog.
                """ </p>

                <p> """
                    storing and retrieving data in this network requires code executed by a portal. 
                    this website is a portal. anyone can make another portal by hosting the code online, 
                    or by downloading it and running it from the command line. 
                    all portals follow the same traces to the same pool of stored data, they never store any data themselves.
                """ </p>

                <p> """
                    all posts are cryptographically signed so that they cannot be fetched if the content is tampered with. 
                    by default, posting is anonymous and posts cannot be edited. 
                    to edit a post, you will need to create an account - a random keypair that you can save locally on your device. 
                    if you are logged in with this keypair and you view a post that you made while logged in, it will have additional edit and delete options.
                """ </p>
            </Collapsible>
        </div>
        <p class="home-warning"> "this project is currently in early development! some features may be incomplete or broken and data may be lost as the storage format undergoes changes." </p>
    }
}
