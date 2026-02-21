use std::str::FromStr;

use intersect_core::{
    api::Intersect,
    log,
    models::{Trace, account::AccountPublic},
};

#[tokio::main]
async fn main() {
    log!("starting...");

    let intersect = Intersect::init().await.unwrap();

    // write account

    let keypair = intersect.connection.with_crypto(|c| c.generate_keypair());
    let name = "evelyn";
    let bio = "hi! <3";
    let home = None;
    let account_public = AccountPublic::new(
        keypair.key(),
        Some(name.to_string()),
        Some(bio.to_string()),
        home,
    )
    .unwrap();

    let trace = intersect.create_account(account_public).await.unwrap();

    let trace_string = trace.to_string();

    // read account

    let read_account_public = intersect
        .read_account(Trace::from_str(&trace_string).unwrap())
        .await
        .unwrap();
    log!("read value: {:?}", read_account_public);

    intersect.close().await;
}
