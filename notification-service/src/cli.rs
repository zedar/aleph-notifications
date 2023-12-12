use std::path::PathBuf;

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

    /// On chain address of KYB registry smart contract
    #[clap(short = 'c')]
    pub sc_address: AccountId,

    /// Path to the contract's metadata json file
    #[clap(short = 'm', default_value = "metadata.json", value_parser = parsing::parse_path)]
    pub sc_metadata: PathBuf,

    /// Commands to interact with Aleph Zero events
    #[clap(subcommand)]
    pub commands: Commands,
}

/// Commands to capture blockchain events
#[derive(Clone, Eq, PartialEq, Debug, Subcommand)]
pub enum Commands {
    /// Capture finalized transfer events for a given on-chain account
    TransferEvent {
        /// Commands defining the target notification channel
        #[clap(subcommand)]
        targets: Targets,
    },

    /// Capture finalized validator rewarded event for a given on-chain account
    RewardedEvent {
        /// Commands definig the target notification channel
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
    },
}

mod parsing {
    use std::{path::PathBuf, str::FromStr};

    use anyhow::{Context, Result};

    pub(super) fn parse_path(path: &str) -> Result<PathBuf> {
        let expanded_path = shellexpand::full(path).context("failed to expand the path")?;
        PathBuf::from_str(&expanded_path).context("failed to parse the path")
    }
}
