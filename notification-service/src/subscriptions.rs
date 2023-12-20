use std::{
    collections::HashMap,
    path::Path,
    sync::{atomic::AtomicBool, Arc, Mutex},
};

use aleph_client::{
    contract::{event::translate_events, ContractInstance, ConvertibleValue},
    AccountId, Connection,
};
use anyhow::{anyhow, bail, Context, Result};
use futures::StreamExt;

/// Represents subscription for on-chain account
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Subscription {
    /// Who registerred new subscription. Events published for this account will result in notifications
    pub for_account: AccountId,

    /// A handle (e.g. chat_id) associated with the user's subscription
    pub channel_handle: String,
}

impl TryFrom<ConvertibleValue> for Subscription {
    type Error = anyhow::Error;

    fn try_from(value: ConvertibleValue) -> Result<Self> {
        let map= match value.0 {
            aleph_client::contract_transcode::Value::Map(map) => map,
            _ =>  bail!("Failed parsing `ConvertibleValue` to `Map<K,V>`. Expected `Map(_)` but instead got: {:?}", value),
        };

        let for_account: AccountId;
        let channel_handle: String;

        match map.ident() {
            Some(x) if x == "ActiveSubscriptionAttr" => {
                match map.get_by_str("for_account") {
                    Some(x) => for_account = ConvertibleValue(x.clone()).try_into()?,
                    _ => bail!(
                        "Failed parsing `for_account`. Expected `AccountId` but got: {:?}",
                        x
                    ),
                }

                match map.get_by_str("external_channel_handle") {
                    Some(x) => channel_handle = ConvertibleValue(x.clone()).try_into()?,
                    _ => bail!(
                        "Failed parsing `external_channel_handle`. Expected `Vec<u8>` but got: {:?}",
                        x
                    ),
                }
                Ok(Subscription {
                    for_account,
                    channel_handle,
                })
            }
            _ => bail!(
                "Expected .ident() to be `ActiveSubscriptionAttr` but got {:?}",
                &map
            ),
        }
    }
}

/// Represents a middleware communicating with Subscriptions smart contract
pub struct Subscriptions {
    /// Terminates event handling loop
    term: Arc<AtomicBool>,

    /// An instance of the smart contract client
    contract: ContractInstance,

    /// A connection to the aleph zero node
    connection: Connection,

    /// List of active subscriptions, each represented as an on-chain account id
    pub active_subscriptions: Arc<Mutex<HashMap<AccountId, Subscription>>>,
}

impl std::fmt::Debug for Subscriptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "active_subscriptions: {:?}", self.active_subscriptions)
    }
}

impl Subscriptions {
    /// Create new instance of the Subscriptions smart contract wrapper
    pub fn new(
        term: Arc<AtomicBool>,
        sc_address: AccountId,
        node_address: &str,
        sc_metadata_path: &Path,
    ) -> Result<Self> {
        let sc_matadata_path = sc_metadata_path
            .to_str()
            .context("Smart contract's metadata not set")?;

        let conn = futures::executor::block_on(Connection::new(node_address));

        Ok(Self {
            term,
            contract: ContractInstance::new(sc_address, sc_matadata_path)?,
            connection: conn,
            active_subscriptions: Arc::new(Mutex::new(HashMap::default())),
        })
    }

    /// Retrieves list of active subscriptions for which notifications should be sent
    pub async fn init_subscriptions(&mut self) -> Result<()> {
        let res: Result<Result<Vec<Subscription>>> = self
            .contract
            .contract_read0(&self.connection, "get_active_subscriptions")
            .await;

        let retrieved_active_subscriptions = res??;

        let mut active_subscriptions = self
            .active_subscriptions
            .lock()
            .map_err(|e| anyhow!(e.to_string()))?;

        for subs in retrieved_active_subscriptions.iter() {
            active_subscriptions.insert(subs.for_account.clone(), subs.clone());
        }

        Ok(())
    }

    /// Listens for smart contract events: NewSubscription, CancelledSubscription, CancelledSubscriptions
    /// For each event either add new subscription or remove active subscriptions.
    pub async fn handle_events(&mut self) -> Result<()> {
        let mut block_sub = self
            .connection
            .as_client()
            .blocks()
            .subscribe_finalized()
            .await
            .context("failed to subscribe for subscriptions smart contract events")?;

        log::info!("aleph-client for subscriptions smart contract is waiting for events...");

        while let Some(Ok(block)) = block_sub.next().await {
            if self.term.load(std::sync::atomic::Ordering::Relaxed) {
                bail!("Subscriptions smart contract event loop terminated")
            }

            let events = match block.events().await {
                Ok(events) => events,
                _ => continue,
            };

            for event in translate_events(
                events.iter(),
                &[&self.contract],
                Some(aleph_client::contract::event::BlockDetails {
                    block_number: block.number(),
                    block_hash: block.hash(),
                }),
            ) {
                if event.is_err() {
                    log::error!(
                        "Error receiving Subscriptions contract event: {:?}",
                        event.err()
                    );
                    continue;
                }
                let event = event.unwrap();
                log::info!("Received smart contract event: {:?}", event);
                match &event.name {
                    Some(n) if n == "NewSubscription" => {
                        let for_account =
                            match self.decode_account_id(event.data.get("for_account")) {
                                Ok(v) => v,
                                Err(err) => {
                                    log::error!(
                                        "AddSubscription event failed to decode for_account: {}",
                                        err
                                    );
                                    continue;
                                }
                            };

                        let channel_handle =
                            match self.decode_string(event.data.get("external_channel_handle")) {
                                Ok(v) => v,
                                Err(err) => {
                                    log::error!(
                                        "AddSubscription event failed to decode channel_handle: {}",
                                        err
                                    );
                                    continue;
                                }
                            };

                        let mut active_subscriptions = match self.active_subscriptions.lock() {
                            Ok(v) => v,
                            Err(err) => {
                                log::error!("Unable to lock active_subscriptions: {:?}", err);
                                continue;
                            }
                        };
                        active_subscriptions.insert(
                            for_account.clone(),
                            Subscription {
                                for_account: for_account.clone(),
                                channel_handle,
                            },
                        );

                        log::info!("New subscription for account: {:?}", for_account);
                    }
                    Some(n) if n == "CancelledSubscription" => {
                        let for_account =
                            match self.decode_account_id(event.data.get("for_account")) {
                                Ok(v) => v,
                                Err(err) => {
                                    log::error!(
                                        "CancelSubscription event failed to decode for_account: {}",
                                        err
                                    );
                                    continue;
                                }
                            };
                        let mut active_subscriptions = match self.active_subscriptions.lock() {
                            Ok(v) => v,
                            Err(err) => {
                                log::error!("Unable to lock active_subscriptions: {:?}", err);
                                continue;
                            }
                        };
                        active_subscriptions.remove(&for_account);

                        log::info!("Cancelled subscription for account: {:?}", for_account);
                    }
                    Some(n) if n == "CancelledSubscriptions" => {
                        let for_accounts =
                            match self.decode_account_ids(event.data.get("for_accounts")) {
                                Ok(v) => v,
                                Err(err) => {
                                    log::error!(
                                    "CancelSubscriptions event failed to decode for_accounts: {}",
                                    err
                                );
                                    continue;
                                }
                            };
                        let mut active_subscriptions = match self.active_subscriptions.lock() {
                            Ok(v) => v,
                            Err(err) => {
                                log::error!("Unable to lock active_subscriptions: {:?}", err);
                                continue;
                            }
                        };
                        for for_account in for_accounts.iter() {
                            active_subscriptions.remove(for_account);
                        }
                    }
                    Some(n) => {
                        log::warn!("Not matched smart contract event name: {}", n);
                        continue;
                    }
                    None => {
                        log::warn!("Undefined smart contract event name");
                        continue;
                    }
                };
            }
        }
        bail!("No more blocks to proceed")
    }

    fn decode_account_id(&self, v: Option<&contract_transcode::Value>) -> Result<AccountId> {
        match v {
            Some(v) => ConvertibleValue(v.clone()).try_into(),
            None => bail!("missing attribute of type AccountId"),
        }
    }

    fn decode_string(&self, v: Option<&contract_transcode::Value>) -> Result<String> {
        match v {
            Some(v) => ConvertibleValue(v.clone()).try_into(),
            None => bail!("missing attribute of type string"),
        }
    }

    fn decode_account_ids(&self, v: Option<&contract_transcode::Value>) -> Result<Vec<AccountId>> {
        let res: Result<Vec<Subscription>> = match v {
            Some(v) => ConvertibleValue(v.clone()).try_into(),
            None => bail!("missing attribute of type Seq<Value>"),
        };
        match res {
            Err(err) => Err(err),
            Ok(v) => Ok(v.into_iter().map(|e| e.for_account).collect::<Vec<_>>()),
        }
    }
}
