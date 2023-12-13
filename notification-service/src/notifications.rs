use aleph_client::{AccountId, Balance};
use anyhow::Result;

pub mod telegram;

/// Formats notification messages
pub trait FormatToString {
    fn format(&self) -> String;
}

/// Represents notification about the transfer `to_account`
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TransferNotification {
    /// The account from which the transfer was made
    pub from_account: AccountId,
    /// The account to which transfer was directed
    pub to_account: AccountId,
    /// Amount of tokens: unit is the smallest token unit, e.g. 1_000_000_000_000 = 1DZERO    
    pub amount: Balance,
}

/// Notification must implement display trait to be prinatable
impl std::fmt::Display for TransferNotification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Notification must implmenet formating to the string message
impl FormatToString for TransferNotification {
    fn format(&self) -> String {
        format!(
            "New transfer from account {:?}, amount {}",
            self.from_account,
            print_with_4_digits(self.amount, 1_000_000_000_000u128) //self.amount as f64 / 1_000_000_000_000_f64
        )
    }
}

fn print_with_4_digits(a: u128, b: u128) -> String {
    let a_mul = a * 10000;
    let div = a_mul / b;

    let frac = div % 10000;
    let rest = div / 10000;

    format!("{}.{:#03}", rest, frac)
}

/// Represents notification about the nominator reward
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RewardedNotification {
    /// The account used by the nominator for stashing
    pub stash_account: AccountId,
    /// Amount of reward: unit is the smallest token unit, e.g. 1_000_000_000_000 = 1DZERO        
    pub amount: Balance,
}

/// Notification must implement display trait to be printable
impl std::fmt::Display for RewardedNotification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Notification must implement formating to the string maessage
impl FormatToString for RewardedNotification {
    fn format(&self) -> String {
        format!(
            "New reward for nominating from account {:?}, amount {}",
            self.stash_account,
            print_with_4_digits(self.amount, 1_000_000_000_000u128) //self.amount as f64 / 1_000_000_000_000_f64
        )
    }
}

/// Alias for bounded notification message. This is an experimental feature that must be enabled with #![feature(trait_alias)]
pub trait NotificationMessage = Clone + FormatToString + std::fmt::Display + Send;

/// Represents channel handle convertible to e.g. Telegram user/chat id
pub struct ChannelHandle(pub String);

/// Sending various notifications
#[async_trait::async_trait]
pub trait NotificationSender {
    /// Sends notification about on-chain event
    async fn send_notification<T: NotificationMessage>(
        &self,
        msg: T,
        channel_handle: ChannelHandle,
    ) -> Result<()>;
}
