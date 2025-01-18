use std::io::Write;

use cli::{run_command, Cli};
use clap::Parser;

use clap_repl::ClapEditor;
use intersect_core::get_routing_context;
use reedline::{DefaultPrompt, DefaultPromptSegment};

mod cli;

#[tokio::main]
async fn main() {
    // make sure intersect is running so we can use all the functionality
    intersect_core::init().await;
    // and wait for a network connection
    get_routing_context().await;

    // try to run a command if there is one
    // else start a repl
    let args = Cli::try_parse();
    match args {
        Ok(command) => run(command).await,
        Err(_) => repl().await,
    }

    // shut down intersect
    intersect_core::shutdown().await;
}

async fn repl() {
    // set up repl
    let mut prompt = DefaultPrompt::default();
    prompt.left_prompt = DefaultPromptSegment::Basic("".to_owned());
    prompt.right_prompt = DefaultPromptSegment::Empty;
    let mut rl = ClapEditor::<Cli>::new_with_prompt(Box::new(prompt), |reed| {
        reed.with_ansi_colors(false)
    });

    loop {
        // Use `read_command` instead of `readline`.
        let Some(command) = rl.read_command() else {
            continue;
        };
        run(command).await;
    }
}

async fn run(command: Cli) {
    match run_command(command).await {
        Ok(()) => {},
        Err(err) => {
            write!(std::io::stdout(), "{err}").map_err(|e| e.to_string()).unwrap();
            std::io::stdout().flush().map_err(|e| e.to_string()).unwrap();
        }
    }
}
