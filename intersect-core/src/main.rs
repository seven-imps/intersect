#![recursion_limit = "256"]
// scratchpad for manual testing. superseded by intersect-cli.

use intersect_core::{
    api::{Intersect, TypedReference},
    debug, log,
    models::Trace,
    veilid::ConnectionParams,
};
use veilid_core::SecretKey;

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

    // let (account_ref, private_key) = intersect
    //     .create_account(Some("evelyn".to_owned()), Some("test bio".to_owned()), None)
    //     .await
    //     .unwrap();

    // debug!(
    //     "account created with reference: {:?} and private key: {:?}",
    //     account_ref, private_key
    // );
    // debug!(
    //     "account: {}",
    //     intersect.account().unwrap().to_unlocked_trace()
    // );

    let account_trace = "26XrzqicitrPi2vBcGbuXyU8FvpTGtexoUUhUZcEHepe8VuijGywRsRZKg7NXPa5oy95RkzMJGBcAzBPQ4JqinAcwFSRDxsYRxPd3B3mXNX1ahVfQmrwqZ2bGGWuL1E8d2K4NuWHMGxY9yXoHR1nz2jcAbhnY82NN3Kz1kWY4sPuj2".parse::<Trace>().unwrap();
    let secret_key = "JHsaNuKGBJV_LpD_jAYUhmYFTQ_adhqZMLfERk9ARMc"
        .parse::<SecretKey>()
        .unwrap();

    intersect
        .login(
            TypedReference::from_trace(account_trace).unwrap(),
            secret_key,
        )
        .await
        .unwrap();

    let account = intersect.open(&intersect.account().unwrap()).await.unwrap();
    let account_view = account.updates.borrow().clone().unwrap();

    debug!("account view: {:?}", account_view);

    intersect.close().await;
}
