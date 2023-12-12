use core::fmt;

use anyhow::{bail, Context, Result};
use teloxide::{prelude::*, types::Recipient};

use super::{ChannelHandle, NotificationMessage, NotificationSender};

/// A Telegram client communicating with a bot
#[derive(Clone, Eq, PartialEq)]
pub struct TelegramBot {
    /// Unique token for Telegram bot
    bot_token: String,
}

impl fmt::Display for TelegramBot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0b{{}}")
    }
}

impl TelegramBot {
    /// Creates new instance of the Telegram bot
    pub fn new(token: String) -> Result<Self> {
        Ok(Self { bot_token: token })
    }

    fn parse_channel_handle(&self, channel_handle: ChannelHandle) -> Result<Recipient> {
        if channel_handle.0.starts_with("channel:") {
            Ok(Recipient::ChannelUsername(
                channel_handle.0.trim_start_matches("channel:").to_string(),
            ))
        } else if channel_handle.0.starts_with("chat_id:") {
            Ok(Recipient::Id(ChatId(
                channel_handle
                    .0
                    .trim_start_matches("chat_id:")
                    .parse::<i64>()?,
            )))
        } else {
            bail!("Unrecognized Telegram handle: {:?}", channel_handle.0)
        }
    }
}

#[async_trait::async_trait]
impl NotificationSender for TelegramBot {
    async fn send_notification<T: NotificationMessage>(
        &self,
        msg: T,
        channel_handle: ChannelHandle,
    ) -> Result<()> {
        log::info!("Sending message to Telegram: {}", msg);

        let bot = Bot::new(&self.bot_token);

        let res = bot
            .send_message(self.parse_channel_handle(channel_handle)?, msg.format())
            .await
            .context("Failed to send message to Telegram bot")?;

        log::debug!("Response from Telegram: {:?}", res);

        Ok(())
    }
}
