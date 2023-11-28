use aleph_client::AccountId;
use clap::{Parser, Subcommand};

/// Utilities to interact with Aleph Zero events
#[derive(Parser, Debug)]
pub struct Cli {
    /// Logging level
    #[clap(
        short = 'l',
        long,
        default_value = "info",
        value_name = "error|warn|info|debug|trace"
    )]
    pub log_level: String,

    /// Webservice endpoint address of the Aleph Zero node
    #[clap(short = 'n', long = "node", default_value = "ws://localhost:9944")]
    pub node_address: String,

    /// Commands to interact with Aleph Zero events
    #[clap(subcommand)]
    pub commands: Commands,
}

/// Commands to capture blockchain events
#[derive(Clone, Eq, PartialEq, Debug, Subcommand)]
pub enum Commands {
    /// Capture reward events for a given chain account
    RewardEvent {
        /// On-chain address for capturing reward events
        #[arg(short = 'a', long)]
        for_account: AccountId,
    },
}
