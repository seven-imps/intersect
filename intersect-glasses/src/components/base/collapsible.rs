use leptos::prelude::*;

#[component]
pub fn Collapsible<F, IV>(
    summary: F,
    children: ChildrenFn,
    #[prop(optional)] default_open: bool,
) -> impl IntoView
where
    F: Fn() -> IV + 'static + Send,
    IV: IntoView + 'static,
{
    view! {
        <details class="collapsible" open=default_open>
            <summary>
                { move || summary() }
            </summary>
            <div class="collapsible-content">
                { children() }
            </div>
        </details>
    }
}
