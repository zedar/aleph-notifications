use core::fmt;

use aleph_client::AccountId;
use anyhow::Result;

pub mod telegram;

/// Represents notification about the transfer extrinsic
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TransferNotification {
    /// The account from which the transfer was made
    pub from_account: AccountId,
    /// The account to which transfer was directed
    pub to_account: AccountId,
    /// Amount of tokens: unit is the smallest token value, e.g. 1_000_000_000_000 = 1DZERO    
    pub amount: u128,
}

impl fmt::Display for TransferNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "New transfer from account {:?}, amount {:.4}",
            self.from_account,
            self.amount as f64 / 1_000_000_000_000u128 as f64
        )
    }
}

/// Sending various notifications
#[async_trait::async_trait]
pub trait NotificationSender {
    /// Sends notification about `Transfer` event
    async fn send_transfer_notification(&self, notif: TransferNotification) -> Result<()>;
}
