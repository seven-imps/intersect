use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
// disable_help_flag: clap's default --help calls process::exit, which would kill the tui
#[command(name = "intersect", no_binary_name = true, disable_help_flag = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Log in: <trace> <secret> for an account. omit args or use 'anon'/'anonymous' for an ephemeral keypair.
    Login {
        /// account trace
        account: Option<String>,
        /// account secret (required if account is specified)
        secret: Option<String>,
    },
    /// Create a new resource
    Create {
        #[command(subcommand)]
        what: CreateCommands,
    },
    /// Fetch a fragment by trace and write it to a file
    Fetch { trace: String, output: PathBuf },
    /// Open a document by trace
    Open { trace: String },
}

#[derive(Debug, Subcommand)]
pub enum CreateCommands {
    /// Create a new account (generates a fresh keypair)
    Account {
        name: Option<String>,
        bio: Option<String>,
    },
    /// Upload a file as a fragment
    Fragment {
        path: PathBuf,
        #[arg(long, default_value = "*/*")]
        mime: String,
    },
}
