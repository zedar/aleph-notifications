use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};

use aleph_client::{
    contract::{ContractInstance, ConvertibleValue},
    AccountId, Connection,
};
use anyhow::{anyhow, bail, Context, Result};

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
    pub fn new(sc_address: AccountId, node_address: &str, sc_metadata_path: &Path) -> Result<Self> {
        let sc_matadata_path = sc_metadata_path
            .to_str()
            .context("Smart contract's metadata not set")?;

        let conn = futures::executor::block_on(Connection::new(node_address));

        Ok(Self {
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

    /// Listens for smart contract
    pub async fn handle_events(&mut self) -> Result<()> {
        Ok(())
    }
}
