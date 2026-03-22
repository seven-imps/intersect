use std::str::FromStr;

use intersect_core::{
    api::{Intersect, TypedReference},
    documents::{AccountDocument, AccountUpdate},
    log,
    models::Trace,
    veilid::{ConnectionParams, with_crypto},
};

#[tokio::main]
async fn main() {
    tokio::task::LocalSet::new().run_until(run()).await;
}

async fn run() {
    let args: Vec<String> = std::env::args().collect();
    let ephemeral = args.contains(&"--ephemeral".to_string());
    let connection_params = ConnectionParams { ephemeral };

    log!("starting... (ephemeral: {})", ephemeral);

    let intersect = Intersect::init(connection_params).await.unwrap();
    let keypair = with_crypto(|c| c.generate_keypair());
    intersect.login(keypair.clone()).unwrap();

    match args
        .iter()
        .find(|a| *a == "create" || *a == "watch")
        .map(|s| s.as_str())
    {
        Some("create") => create(&intersect).await,
        Some("watch") => {
            let trace_str = args
                .iter()
                .skip_while(|a| *a != "watch")
                .nth(1)
                .expect("usage: watch <trace>");
            watch(&intersect, trace_str).await;
        }
        _ => {
            eprintln!("usage: intersect-core [--ephemeral] create|watch <trace>");
        }
    }

    intersect.close().await;
}

async fn create(intersect: &Intersect) {
    let typed_ref = intersect
        .create_account(Some("evelyn".to_string()), Some("hi! <3".to_string()), None)
        .await
        .unwrap();
    log!("created account");
    log!("trace: {}", typed_ref.to_unlocked_trace());

    loop {
        log!("enter new name (blank to quit):");
        let mut new_name = String::new();
        std::io::stdin().read_line(&mut new_name).unwrap();
        let new_name = new_name.trim().to_string();

        if new_name.is_empty() {
            break;
        }

        intersect
            .update(&typed_ref, AccountUpdate::Name(Some(new_name.clone())))
            .await
            .unwrap();
        log!("updated name to '{}'", new_name);
    }
}

async fn watch(intersect: &Intersect, trace_str: &str) {
    let typed_ref = TypedReference::<AccountDocument>::try_from(
        Trace::from_str(trace_str).expect("invalid trace"),
    )
    .expect("trace must be unlocked and of type Account");
    log!("watching account...");
    let (_, mut rx) = intersect.open(&typed_ref).await.unwrap();
    loop {
        match rx.borrow_and_update().as_ref() {
            Ok(view) => log!("view: {:?}", view.public),
            Err(e) => log!("error: {}", e),
        }
        if rx.changed().await.is_err() {
            break;
        }
    }
}
