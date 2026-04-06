#![recursion_limit = "256"]
// scratchpad for manual testing. superseded by intersect-cli.

use intersect_core::{ConnectionParams, Intersect, log};

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
    intersect.wait_for_attachment().await;

    // add scratchpad code here

    intersect.close().await;
}
