use core::fmt;

use anyhow::{Context, Result};
use teloxide::{prelude::*, types::Recipient};

use super::NotificationSender;

/// A Telegram client communicating with a bot
#[derive(Clone, Eq, PartialEq)]
pub struct TelegramBot {
    /// Unique token for Telegram bot
    bot_token: String,
    /// Sender unique identifier (User, Group chat) of the Telegram service
    chat_id: i64,
}

impl fmt::Display for TelegramBot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0b{:?}", self.chat_id)
    }
}

impl TelegramBot {
    /// Creates new instance of the Telegram bot
    pub fn new(token: String, chat_id: i64) -> Result<Self> {
        Ok(Self {
            bot_token: token,
            chat_id,
        })
    }
}

#[async_trait::async_trait]
impl NotificationSender for TelegramBot {
    async fn send_transfer_notification(&self, notif: super::TransferNotification) -> Result<()> {
        let txt = notif.to_string();
        let bot = Bot::new(&self.bot_token);

        log::info!("Sending message to telegram: {}", notif);

        let res = bot
            .send_message(Recipient::Id(ChatId(self.chat_id)), txt)
            .await
            .context("Failed to send message to Telegram bot")?;

        log::debug!("Response from Telegram: {:?}", res);

        Ok(())
    }
}
