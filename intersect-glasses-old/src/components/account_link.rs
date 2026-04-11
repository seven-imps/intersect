// use intersect_core::Shard;
// use leptos::*;

// use crate::pages::view_link;

// #[component]
// pub fn AccountLink(
//     #[prop(into)] link: Shard,
//     #[prop(into, optional)] class: String,
// ) -> impl IntoView {
//     let title = "title text";
//     let (href, state) = view_link(&link);
//     let class = format!("link link-intersect {}", class);

//     view! {
//         <A class=class href=href state=state.into() attr:title=title>{text}</A>
//     }
// }
