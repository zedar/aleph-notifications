use std::sync::{atomic::AtomicBool, Arc};

use aleph_client::{
    api::{balances::events::Transfer, staking::events::Rewarded},
    AccountId, Connection,
};
use anyhow::{bail, Context, Result};
use futures::StreamExt;

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

    /// Logs every reward event for a given on-chain address
    pub async fn log_reward_events(&self, conn: Connection, _for_account: AccountId) -> Result<()> {
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
                if let Ok(Some(evt)) = event.as_event::<Rewarded>() {
                    log::info!("Received Rewarded event: {:?}", evt);
                } else if let Ok(Some(evt)) = event.as_event::<Transfer>() {
                    log::info!("Received Transfer event: {:?}", evt);
                }
            }
        }

        bail!("No more blocks to proceed")
    }
}
