use aleph_client::AccountId;
use anyhow::{bail, Result};
use clap::{Args, Parser, Subcommand};
use teloxide::types::{ChatId, Recipient};

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
    /// Capture finalized transfer events for a given on-chain account
    TransferEvent {
        /// On-chain address for capturing events
        #[arg(short = 'a', long)]
        to_account: AccountId,

        /// Commands defining the target notification channel
        #[clap(subcommand)]
        targets: Targets,
    },

    /// Capture finalized validator rewarded event for a given on-chain account
    RewardedEvent {
        /// On-chain address for capturing events
        #[arg(short = 'a', long)]
        for_account: AccountId,

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

        #[clap(flatten)]
        user: TelegramUser,
    },
}

#[derive(Debug, Clone, Eq, PartialEq, Args)]
#[group(required = true, multiple = false)]
pub struct TelegramUser {
    /// Telegram channel - at least one of chat_id|chat_username is required
    #[arg(short = 'c', long)]
    chat_id: Option<i64>,
    /// Telegram username - at least one of chat_id|char_username is required
    #[arg(short = 'u', long)]
    channel_username: Option<String>,
}

impl TryInto<Recipient> for TelegramUser {
    type Error = anyhow::Error;
    fn try_into(self) -> Result<Recipient> {
        let recipient = match self.chat_id {
            Some(chat_id) => Recipient::Id(ChatId(chat_id)),
            _ => match self.channel_username {
                Some(chat_username) => Recipient::ChannelUsername(chat_username),
                _ => bail!("missing Telegram's chat_id|username"),
            },
        };
        Ok(recipient)
    }
}
