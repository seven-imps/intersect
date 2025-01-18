use leptos::*;

#[component]
pub fn IfSome<T, F, IV>(
    #[prop(into)] signal: Signal<Option<T>>,
    view: F,
    #[prop(optional, into)] fallback: ViewFn,
) -> impl IntoView
where
    T: Clone + 'static,
    F: Fn(T) -> IV + 'static,
    IV: IntoView,
{
    move || {
        signal
            .get()
            .map_or_else(|| fallback.run(), |value| view(value).into_view())
    }
}
