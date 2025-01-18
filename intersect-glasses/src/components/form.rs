use leptos::*;

use crate::components::empty_view;

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
    let children = store_value(children);
    let error_signal = create_rw_signal("".to_string());

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default(); // no redirect on submit

        let valid = validate.call(());
        match valid {
            Ok(value) => {
                error_signal.set("".to_string());
                on_submit.call(value);
            }
            Err(error) => error_signal.set(error.to_string()),
        }
    };

    let form_errors = move || {
        let error = error_signal.get();
        if !error.is_empty() {
            view! {
                <div class="form-errors">
                    <p> {error} </p>
                </div>
            }
            .into_view()
        } else {
            empty_view()
        }
    };

    let form_shell = |children| {
        if pwd_manager_workarounds {
            view! {
                // some magic to make password managers happy
                // this is Important™
                <form class="form-main" action="" method="post" autocomplete="on" on:submit=submit>
                    {children}
                </form>
            }
        } else {
            view! {
                <form class="form-main" on:submit=submit>
                    {children}
                </form>
            }
        }
    };

    form_shell(view! {
        {form_errors}
        {children.with_value(|children| children())}
    })
}
