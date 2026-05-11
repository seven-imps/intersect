#[macro_export]
macro_rules! make_action {
    // named and typed param
    (move |$param:ident: $param_type:ty| $code:expr) => {
        create_action(move |$param: $param_type| {
            let $param = $param.clone();
            async move { $code }
        })
    };

    // ignored param
    (move |_| $code:expr) => {
        create_action(move |_: &()| async move { $code })
    };
}
