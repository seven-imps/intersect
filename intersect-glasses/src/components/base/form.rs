use leptos::prelude::*;

#[component]
pub fn Form<T>(
    children: ChildrenFn,
    #[prop(into)] validate: Callback<(), Result<T, anyhow::Error>>,
    #[prop(into)] on_submit: Callback<T>,
    #[prop(optional)] pwd_manager_workarounds: bool,
) -> impl IntoView
where
    T: 'static,
{
    let children = StoredValue::new(children);
    let error = RwSignal::new(String::new());

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default(); // no redirect on submit
        match validate.run(()) {
            Ok(value) => {
                error.set(String::new());
                on_submit.run(value);
            }
            Err(err) => error.set(err.to_string()),
        }
    };

    view! {
        <form
            class="form-main"
            on:submit=submit
            // some magic to make password managers happy
            // this is Important™
            action=pwd_manager_workarounds.then_some("")
            method=pwd_manager_workarounds.then_some("post")
            autocomplete=pwd_manager_workarounds.then_some("on")
        >
            <Show when=move || !error.get().is_empty()>
                <div class="form-errors">
                    <p>{move || error.get()}</p>
                </div>
            </Show>
            {children.with_value(|children| children())}
        </form>
    }
}
