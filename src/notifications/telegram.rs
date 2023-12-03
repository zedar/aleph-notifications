use core::fmt;

use anyhow::{Context, Result};
use teloxide::{prelude::*, types::Recipient};

use super::{NotificationMessage, NotificationSender};

/// A Telegram client communicating with a bot
#[derive(Clone, Eq, PartialEq)]
pub struct TelegramBot {
    /// Unique token for Telegram bot
    bot_token: String,
    /// Sender unique identifier (User, Group chat) of the Telegram service
    recipient: Recipient,
}

impl fmt::Display for TelegramBot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0b{:?}", self.recipient)
    }
}

impl TelegramBot {
    /// Creates new instance of the Telegram bot
    pub fn new(token: String, user: Recipient) -> Result<Self> {
        Ok(Self {
            bot_token: token,
            recipient: user,
        })
    }
}

#[async_trait::async_trait]
impl NotificationSender for TelegramBot {
    async fn send_notification<T: NotificationMessage>(&self, msg: T) -> Result<()> {
        log::info!("Sending message to Telegram: {}", msg);

        let bot = Bot::new(&self.bot_token);

        let res = bot
            .send_message(self.recipient.clone(), msg.format())
            .await
            .context("Failed to send message to Telegram bot")?;

        log::debug!("Response from Telegram: {:?}", res);

        Ok(())
    }
}
