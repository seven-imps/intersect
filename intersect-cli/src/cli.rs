use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "intersect", no_binary_name = true, disable_help_flag = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Create a new resource
    Create {
        #[command(subcommand)]
        what: CreateCommands,
    },
    /// Open a document by trace
    Open {
        trace: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum CreateCommands {
    /// Create a new account (generates a fresh keypair)
    Account {
        name: Option<String>,
        bio: Option<String>,
    },
}
