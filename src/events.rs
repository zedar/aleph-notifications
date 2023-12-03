use std::sync::{atomic::AtomicBool, Arc};

use aleph_client::{
    api::{balances::events::Transfer, staking::events::Rewarded},
    AccountId, Connection,
};
use anyhow::{bail, Context, Result};
use futures::StreamExt;
use subxt::events::StaticEvent;

use crate::notifications::{NotificationMessage, NotificationSender};

/// Events subsription logic
#[derive(Debug)]
pub struct Events {
    /// Terminates event handling loop
    term: Arc<AtomicBool>,
}

impl Events {
    /// Creates new instance of the events handler
    pub fn new(term: Arc<AtomicBool>) -> Result<Self> {
        Ok(Self { term })
    }

    /// Sends notification about every transfer event for a given on-chain address
    pub async fn send_transfer_event_notification(
        &self,
        conn: Connection,
        stash_account: AccountId,
        notifier: &impl NotificationSender,
    ) -> Result<()> {
        self.send_event_notification(
            conn,
            |evt: &Transfer| {
                if evt.to.0 == stash_account {
                    true
                } else {
                    false
                }
            },
            |evt: &Transfer| crate::notifications::TransferNotification {
                from_account: evt.from.0.clone(),
                to_account: evt.to.0.clone(),
                amount: evt.amount,
            },
            notifier,
        )
        .await
    }

    /// Sends notification about rewarded events associated with a given stash account
    pub async fn send_rewarded_event_notification(
        &self,
        conn: Connection,
        stash_account: AccountId,
        notifier: &impl NotificationSender,
    ) -> Result<()> {
        self.send_event_notification(
            conn,
            |evt: &Rewarded| {
                if evt.stash.0 == stash_account {
                    true
                } else {
                    false
                }
            },
            |evt: &Rewarded| crate::notifications::RewardedNotification {
                stash_account: evt.stash.0.clone(),
                amount: evt.amount,
            },
            notifier,
        )
        .await
    }

    async fn send_event_notification<
        T: StaticEvent,
        M: NotificationMessage,
        P: Fn(&T) -> bool + Send,
        C: Fn(&T) -> M,
    >(
        &self,
        conn: Connection,
        predicate: P,
        converter: C,
        notifier: &impl NotificationSender,
    ) -> Result<()> {
        let mut block_sub = conn
            .as_client()
            .blocks()
            .subscribe_finalized()
            .await
            .context("Failed to subscribe to the finalized block stream")?;

        log::info!("aleph-client waiting for events ...");

        while let Some(Ok(block)) = block_sub.next().await {
            if self.term.load(std::sync::atomic::Ordering::Relaxed) {
                bail!("Rewards events terminated")
            }
            let events = match block.events().await {
                Ok(events) => events,
                _ => continue,
            };
            for event in events.iter() {
                let event = event.context("Failed to obtain event from the block")?;
                if let Ok(Some(evt)) = event.as_event::<T>() {
                    if !predicate(&evt) {
                        continue;
                    }
                    let msg = converter(&evt);
                    let res = notifier.send_notification(msg.clone()).await;
                    match res {
                        Err(err) => log::error!(
                            "Error sending notification for event: {}, error: {}",
                            msg.to_string(),
                            err
                        ),
                        Ok(_) => continue,
                    };
                }
            }
        }

        bail!("No more blocks to proceed")
    }
}
