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
    TransferEvent {
        /// On-chain address for capturing reward events
        #[arg(short = 'a', long)]
        to_account: AccountId,

        /// Commands defining the target of the notifications
        #[clap(subcommand)]
        targets: Targets,
    },
}

/// Commands to define target channel for notifications
#[derive(Debug, Clone, Eq, PartialEq, Subcommand)]
pub enum Targets {
    /// Notification are sent to the Telegram bot
    Telegram {
        /// Telegram bot token
        #[arg(short = 't', long)]
        token: String,
        /// Telegram channel
        #[arg(short = 'c', long)]
        chat_id: i64,
    },
}
