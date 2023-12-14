#![cfg_attr(not(feature = "std"), no_std, no_main)]

// Notes:
// Block time (Aleph Zero): The average time it takes the network to generate a new block. The block time of Aleph Zero is set to 1 second.

#[ink::contract]
mod subscriptions {

    use ink::{
        prelude::{format, string::String, vec::Vec, *},
        storage::Mapping,
    };

    pub const BLOCKS_PER_WEEK: u32 = 3600 * 24 * 7;
    pub const BLOCKS_PER_MONTH: u32 = 3600 * 24 * 7 * 30;

    /// Defines subscription payment interval
    #[derive(Debug, Clone, Copy, PartialEq, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum PaymentInterval {
        Week,
        Month,
    }

    /// Subscription data
    #[derive(Debug, Clone, scale::Encode, scale::Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Subscription {
        /// Declared payment interval
        payment_interval: PaymentInterval,
        /// Number of declared payment intervalas
        declared_payment_intervals: u32,
        /// Number of already paid intervals
        paid_intervals: u32,
        /// Price per interval calculated at the time of subscription registration
        /// Units - the smallest unit, e.g. 1_000_000_000_000 = 1DZERO, 1TZERO, 1AZERO
        price_per_interval: Balance,
        /// Registered at
        registered_at: BlockNumber,
        /// Last payment at
        last_payment_at: BlockNumber,
        /// External channel handle specific for the subscription, e.g. Telegram channel ID
        external_channel_handle: String,
    }

    /// Active subscription attributes to be exposed externally
    #[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct ActiveSubscriptionAttr {
        /// Who registerred new subscription. Events published for this account will result in notifications
        for_account: AccountId,

        /// A handle (e.g. chat_id) associated with the user's subscription
        external_channel_handle: Vec<u8>,
    }

    /// Defines the storage layout of this smart contract.
    #[ink(storage)]
    pub struct Subscriptions {
        /// Only owner of this smart contract can start payment settlements and can transfer ownership
        owner: AccountId,
        /// Price per subscription per block that can be translated to a payment interval
        /// Units - the smallest unit, e.g. 1_000_000_000_000 = 1DZERO, 1TZERO, 1AZERO
        price_per_block: Balance,
        /// Registered and active subscriptions
        subscriptions: Mapping<AccountId, Subscription>,
        /// List of active subscriptions
        active_subscriptions: Vec<AccountId>,
    }

    /// Errors returned by this smart contract
    #[derive(Debug, Clone, Eq, PartialEq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Returned when the calling account is not authorized to perform an action
        NotAuthorized,
        /// Returned when subscription for a given account already registerred
        AlreadyRegisterred(AccountId),
        /// Returned when too low (==0) number of intervals to pay has been provided
        InvalidIntervalsToPay(u32),
        /// Costs of subscription too high. Required value passed as an error parameter
        SubscriptionCostTooHigh(Balance),
        /// Returned when subscription does not exists for a given account
        NotRegisterred(AccountId),
        /// Returned when new owner is the same as the old one
        NewOwnerMustBeDifferent,
        /// Returned when subscription not found but is on the list of active subscriptions
        InconsistentSubscriptionData(AccountId),
        /// Ink! error can be converted to this smart contract errors
        InkEnvFailure(String),
    }

    /// Converts ink::env::Error to this smart contract error
    impl From<ink::env::Error> for Error {
        fn from(value: ink::env::Error) -> Self {
            Error::InkEnvFailure(format!("{:?}", value))
        }
    }

    /// Event emitted when a new subscription is added
    #[ink(event)]
    pub struct NewSubscription {
        /// Who registerred new subscription. Events published for this account will result in notifications
        #[ink(topic)]
        for_account: AccountId,

        /// A handle (e.g. chat_id) associated with the user's subscription
        external_channel_handle: Vec<u8>,
    }

    /// Event emitted on subscription cancellation
    #[ink(event)]
    pub struct CancelledSubscription {
        /// Whe cancelled the subscription.
        #[ink(topic)]
        for_account: AccountId,
    }

    /// Event emitted on payment settelment, when there is at least one subscription to be cancelled
    #[ink(event)]
    pub struct CancelledSubscriptions {
        for_accounts: Vec<AccountId>,
    }

    impl Subscriptions {
        /// Creates new instance of this smart contract with empty list of subscriptions.
        /// The caller of this function becomes an owner of the subscriptions registry.
        /// Only the owner can start payment settlement and transfer an ownership.
        /// Parameters:
        /// * `price_per_block` - price the subscriber needs to pay for the number of blocks translated to the payment interval.
        #[ink(constructor)]
        pub fn new(price_per_block: Balance) -> Self {
            Self {
                owner: Self::env().caller(),
                price_per_block,
                subscriptions: Mapping::default(),
                active_subscriptions: Vec::default(),
            }
        }

        /// Registers new subscrption for a caller and a given time period.
        /// Parameters:
        /// * payment_interval - one of week|month
        /// * intervals_to_pay - number of paid intervales declared by the caller
        /// * external_channel_handle_id - external identifier, specific for the external channel, used by the notification service
        /// Events:
        /// * NewSubscription
        /// Fails:
        /// * when subscription is already registerred
        /// * when invalid payment interval
        /// * when not enough token value transferred to the smart contract call
        #[ink(message, payable)]
        pub fn add_subscription(
            &mut self,
            payment_interval: PaymentInterval,
            intervals_to_pay: u32,
            external_channel_handle: String,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            if self.subscriptions.get(caller).is_some() {
                return Err(Error::AlreadyRegisterred(caller));
            }

            if intervals_to_pay == 0 {
                return Err(Error::InvalidIntervalsToPay(intervals_to_pay));
            }

            let curr_block = self.env().block_number();
            let price_per_interval = self.price_per_interval(&payment_interval);
            let subscription = Subscription {
                payment_interval,
                declared_payment_intervals: intervals_to_pay,
                paid_intervals: 1,
                price_per_interval,
                registered_at: curr_block,
                last_payment_at: curr_block,
                external_channel_handle: external_channel_handle.clone(),
            };

            // Check hom much tokens have been transferred as part of the transaction
            let transferred_value = self.env().transferred_value();
            if transferred_value < price_per_interval * intervals_to_pay as u128 {
                return Err(Error::SubscriptionCostTooHigh(
                    price_per_interval * intervals_to_pay as u128,
                ));
            }

            // Transfer one interval payment to the contract's owner. The tokens needed for the remaining paiments will stay in the contract
            self.transfer_to_owner(price_per_interval);

            // If user transferred more than expected
            self.reimburse(
                caller,
                transferred_value - price_per_interval * intervals_to_pay as u128,
            );

            self.subscriptions.insert(caller, &subscription);
            self.active_subscriptions.push(caller);

            self.env().emit_event(NewSubscription {
                for_account: caller,
                external_channel_handle: external_channel_handle.into_bytes(),
            });

            Ok(())
        }

        /// Cancels subscription associated with a caller.
        /// All remaining tokens are transferred back to the caller.
        /// Events:
        /// * CancelledSubscription
        /// Fails:
        /// * SubscriptionNotFound - when there is no subscription associated with the caller's account
        #[ink(message, payable)]
        pub fn cancel_subscription(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();

            let subscription = self
                .subscriptions
                .get(caller)
                .ok_or(Error::NotRegisterred(caller))?;

            // Transfer remaining token value
            let mut to_return: Balance = 0;
            if subscription.declared_payment_intervals > subscription.paid_intervals {
                to_return = subscription.price_per_interval
                    * (subscription.declared_payment_intervals - subscription.paid_intervals)
                        as u128;
            }

            // Get all transferred tokens. We need to return them.
            let transferred_value = self.env().transferred_value();
            to_return += transferred_value;

            // If there is something to return
            if to_return > 0 {
                self.reimburse(caller, to_return);
            }

            self.subscriptions.remove(caller);
            self.active_subscriptions.retain(|acct| acct != &caller);

            self.env().emit_event(CancelledSubscription {
                for_account: caller,
            });

            Ok(())
        }

        /// Retrieves a list of active subscriptions.
        /// Returns:
        /// * list of active subscriptions
        /// Fails
        /// * when there is an inconsistent subscription data
        #[ink(message)]
        pub fn get_active_subscriptions(&self) -> Result<Vec<ActiveSubscriptionAttr>, Error> {
            let mut subs = vec![];
            for acct_id in &*self.active_subscriptions {
                let sub = self
                    .subscriptions
                    .get(acct_id)
                    .ok_or(Error::InconsistentSubscriptionData(*acct_id))?;
                subs.push(ActiveSubscriptionAttr {
                    for_account: *acct_id,
                    external_channel_handle: sub.external_channel_handle.into_bytes(),
                });
            }
            Ok(subs)
        }

        /// Run payment settlement for the next subscription round.
        /// For each active subscription check:
        /// * is it still active
        /// * does it have enough funds for the next interval
        /// If above rules are not fulfilled subscription is automatically cancelled
        #[ink(message, payable)]
        pub fn payment_settlement(&mut self) -> Result<(), Error> {
            self.authorized(self.env().caller())?;

            let mut subs_to_cancel: Vec<AccountId> = vec![];

            let curr_block = self.env().block_number();

            for acct_id in &*self.active_subscriptions {
                let mut s = self
                    .subscriptions
                    .get(acct_id)
                    .ok_or(Error::InconsistentSubscriptionData(*acct_id))?;
                // calculate number of to pay intervals
                let mut to_pay_intervals =
                    self.to_pay_intervals(s.payment_interval, curr_block, s.last_payment_at);
                // check if there is something to pay
                if to_pay_intervals == 0 {
                    continue;
                }
                // if founds are not sufficient to pay all intervals to pay, transfer the remaining funds
                if s.declared_payment_intervals - s.paid_intervals < to_pay_intervals {
                    to_pay_intervals = s.declared_payment_intervals - s.paid_intervals;
                }

                let to_pay = s.price_per_interval * to_pay_intervals as u128;

                self.transfer_to_owner(to_pay);

                s.paid_intervals += to_pay_intervals;
                s.last_payment_at = curr_block;

                if s.declared_payment_intervals == s.paid_intervals {
                    // add subscription to the list of to be cancelled subsccriptions
                    subs_to_cancel.push(*acct_id);
                } else {
                    self.subscriptions.insert(acct_id, &s);
                }
            }

            // cancel subscriptions
            for acct_id in &*subs_to_cancel {
                self.subscriptions.remove(acct_id);
                self.active_subscriptions.retain(|id| acct_id != id);
            }
            if !subs_to_cancel.is_empty() {
                // emit an event with a list of cancelled subscriptions
                self.env().emit_event(CancelledSubscriptions {
                    for_accounts: subs_to_cancel,
                });
            }
            Ok(())
        }

        /// Transfers ownership to a new owner. Only current owner is allowed to call it.
        /// Parameters:
        /// * `new_owner` - new smart contract owner account
        ///
        /// Fails:
        /// * caller is not an owner of the smart contract
        /// * caller and new owner is the same account
        #[ink(message)]
        pub fn transfer_ownership(&mut self, new_owner: AccountId) -> Result<(), Error> {
            let caller = self.env().caller();
            self.authorized(caller)?;

            if new_owner == self.owner {
                return Err(Error::NewOwnerMustBeDifferent);
            }

            self.owner = new_owner;
            Ok(())
        }

        /// Modifies the code which is used to execute calls to this contract address (`AccountId`).
        ///
        /// We use this to upgrade the contract logic. We don't do any authorization here, any caller
        /// can execute this method. In a production contract you would do some authorization here.
        #[ink(message)]
        pub fn set_code(&mut self, code_hash: [u8; 32]) -> Result<(), Error> {
            self.authorized(self.env().caller())?;
            ink::env::set_code_hash(&code_hash).unwrap_or_else(|err| {
                panic!(
                    "Failed to `set_code_hash` to {:?} due to {:?}",
                    code_hash, err
                )
            });
            ink::env::debug_println!("Switched code hash to {:?}.", code_hash);
            Ok(())
        }

        fn authorized(&self, caller: AccountId) -> Result<(), Error> {
            if caller != self.owner {
                return Err(Error::NotAuthorized);
            }
            Ok(())
        }

        fn price_per_interval(&self, payment_interval: &PaymentInterval) -> Balance {
            self.price_per_block
                * match payment_interval {
                    PaymentInterval::Week => BLOCKS_PER_WEEK as u128,
                    PaymentInterval::Month => BLOCKS_PER_MONTH as u128,
                }
        }

        fn to_pay_intervals(
            &self,
            payment_interval: PaymentInterval,
            curr_block: BlockNumber,
            last_payment_at: BlockNumber,
        ) -> u32 {
            (curr_block - last_payment_at)
                / match payment_interval {
                    PaymentInterval::Week => BLOCKS_PER_WEEK,
                    PaymentInterval::Month => BLOCKS_PER_MONTH,
                }
        }

        /// Transfers amount of tokens from the contract's account to the owner account.
        fn transfer_to_owner(&self, amount: Balance) {
            if Self::env().transfer(self.owner, amount).is_err() {
                panic!("failed to transfer tokens to owner")
            }
        }

        /// Reimburses the caller with overpaid tokens.
        /// Panics if the transfer fails - this means this contract's balance is
        /// too low which means something went wrong.
        fn reimburse(&self, recipient: AccountId, amount: Balance) {
            if Self::env().transfer(recipient, amount).is_err() {
                panic!("failed to reimburse the caller")
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use ink::{
            env::test::{recorded_events, EmittedEvent},
            reflect::ContractEventBase,
        };

        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        pub const ONE_TOKEN: Balance = 1_000_000_000_000;
        pub const ONE_WEEK_TOKENS: Balance = 604_800;

        // Alias for wrapper around all events in this smart contract generated by ink!
        type Event = <Subscriptions as ContractEventBase>::Type;

        /// We test a simple use case of our contract.
        #[ink::test]
        fn it_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(accounts.bob, 0);
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let mut subscriptions = Subscriptions::new(1u128);

            assert_eq!(&subscriptions.owner, &accounts.bob);
            assert_eq!(subscriptions.price_per_block, 1u128);

            // prepare balance for the caller
            ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(
                accounts.charlie,
                2 * ONE_TOKEN,
            );
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            ink::env::test::transfer_in::<ink::env::DefaultEnvironment>(ONE_TOKEN);
            // add subscription
            subscriptions
                .add_subscription(PaymentInterval::Week, 1, "1111".to_string())
                .unwrap();
            assert!(subscriptions.subscriptions.contains(accounts.charlie));
            assert!(subscriptions
                .active_subscriptions
                .contains(&accounts.charlie));

            // alice, an owner of the contract should get payment
            assert_eq!(
                ONE_WEEK_TOKENS,
                ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(accounts.bob)
                    .unwrap()
            );
            // overpaid tokens should be returned to charlie
            assert_eq!(
                2 * ONE_TOKEN - ONE_WEEK_TOKENS,
                ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(
                    accounts.charlie
                )
                .unwrap()
            );

            // test recorded events
            let events = recorded_events().collect::<Vec<_>>();
            assert_new_subscription(&events[0], accounts.charlie, "1111".to_string());
        }

        #[ink::test]
        fn cancel_subscription_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // setup Bob as a contract owner
            ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(accounts.bob, 0);
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let mut subscriptions = Subscriptions::new(1u128);

            // prepare balance for the Charlie as the contract caller
            ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(
                accounts.charlie,
                ONE_TOKEN,
            );
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            ink::env::test::transfer_in::<ink::env::DefaultEnvironment>(ONE_TOKEN);
            // add subscription
            subscriptions
                .add_subscription(PaymentInterval::Week, 1, "1111".to_string())
                .unwrap();
            assert!(subscriptions.subscriptions.contains(accounts.charlie));
            assert!(subscriptions
                .active_subscriptions
                .contains(&accounts.charlie));

            // Charlie cancels subscription
            subscriptions.cancel_subscription().unwrap();
            assert!(!subscriptions.subscriptions.contains(accounts.charlie));
            assert!(!subscriptions
                .active_subscriptions
                .contains(&accounts.charlie));

            // test if remaining tokens are returned to the Charlie
            assert_eq!(
                ONE_TOKEN - ONE_WEEK_TOKENS,
                ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(
                    accounts.charlie
                )
                .unwrap()
            );
            // test recorded events
            let events = recorded_events().collect::<Vec<_>>();
            assert_new_subscription(&events[0], accounts.charlie, "1111".to_string());
            assert_cancelled_subscription(&events[1], accounts.charlie);
        }

        #[ink::test]
        fn get_active_subscriptions_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut subscriptions = Subscriptions::new(0u128);

            // prepare balance for the Charlie as the contract caller
            ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(
                accounts.charlie,
                ONE_TOKEN,
            );
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            ink::env::test::transfer_in::<ink::env::DefaultEnvironment>(ONE_TOKEN);
            // add subscription
            subscriptions
                .add_subscription(PaymentInterval::Week, 1, "1111".to_string())
                .unwrap();
            assert!(subscriptions.subscriptions.contains(accounts.charlie));
            assert!(subscriptions
                .active_subscriptions
                .contains(&accounts.charlie));

            // test list of active subscriptions
            assert_eq!(
                subscriptions.get_active_subscriptions().unwrap(),
                vec![ActiveSubscriptionAttr {
                    for_account: accounts.charlie,
                    external_channel_handle: "1111".as_bytes().to_vec()
                }]
            );
        }

        #[ink::test]
        fn payment_settlement_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut subscriptions = Subscriptions::new(1u128);

            // register subscription for Bob
            ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(
                accounts.bob,
                ONE_TOKEN,
            );
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            ink::env::test::transfer_in::<ink::env::DefaultEnvironment>(ONE_TOKEN);
            subscriptions
                .add_subscription(PaymentInterval::Week, 2, "1111".to_string())
                .unwrap();
            // register subscription for Charlie
            ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(
                accounts.charlie,
                3 * ONE_TOKEN,
            );
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            ink::env::test::transfer_in::<ink::env::DefaultEnvironment>(ONE_TOKEN);
            // add subscription
            subscriptions
                .add_subscription(PaymentInterval::Week, 3, "2222".to_string())
                .unwrap();

            assert!(subscriptions.subscriptions.contains(accounts.bob));
            assert!(subscriptions.active_subscriptions.contains(&accounts.bob));
            assert_eq!(
                subscriptions
                    .subscriptions
                    .get(accounts.bob)
                    .unwrap()
                    .paid_intervals,
                1
            );
            assert!(subscriptions.subscriptions.contains(accounts.charlie));
            assert!(subscriptions
                .active_subscriptions
                .contains(&accounts.charlie));
            assert_eq!(
                subscriptions
                    .subscriptions
                    .get(accounts.charlie)
                    .unwrap()
                    .paid_intervals,
                1
            );

            // advance one block and execute payment settlement
            ink::env::test::advance_block::<ink::env::DefaultEnvironment>();
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            assert!(subscriptions.payment_settlement().is_ok());

            assert_eq!(
                subscriptions
                    .subscriptions
                    .get(accounts.bob)
                    .unwrap()
                    .paid_intervals,
                1
            );
            assert_eq!(
                subscriptions
                    .subscriptions
                    .get(accounts.charlie)
                    .unwrap()
                    .paid_intervals,
                1
            );

            // advancce one week of blocks
            for _ in 0..BLOCKS_PER_WEEK {
                ink::env::test::advance_block::<ink::env::DefaultEnvironment>();
            }
            assert!(subscriptions.payment_settlement().is_ok());
            assert!(subscriptions.subscriptions.get(accounts.bob).is_none());
            assert!(subscriptions.subscriptions.get(accounts.charlie).is_some());
            assert_eq!(
                subscriptions
                    .subscriptions
                    .get(accounts.charlie)
                    .unwrap()
                    .paid_intervals,
                2
            );

            // test emitted events
            let events = recorded_events().collect::<Vec<_>>();
            assert_new_subscription(&events[0], accounts.bob, "1111".to_string());
            assert_new_subscription(&events[1], accounts.charlie, "2222".to_string());
            assert_cancelled_subscriptions(&events[2], vec![accounts.bob]);
        }

        #[ink::test]
        fn only_owner_allowed_to_transfer_ownership() {
            // given
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut subscriptions = Subscriptions::new(1u128);
            assert_eq!(subscriptions.owner, accounts.alice);

            // transfer ownership to bob
            assert!(subscriptions.transfer_ownership(accounts.bob).is_ok());
            assert_eq!(subscriptions.owner, accounts.bob);
        }

        fn assert_new_subscription(
            event: &EmittedEvent,
            expected_for_account: AccountId,
            expected_external_channel_handle: String,
        ) {
            let decoded_event = <Event as scale::Decode>::decode(&mut &event.data[..])
                .expect("invalid event buffer");
            if let Event::NewSubscription(NewSubscription {
                for_account,
                external_channel_handle,
            }) = decoded_event
            {
                assert_eq!(for_account, expected_for_account);
                assert_eq!(
                    external_channel_handle,
                    expected_external_channel_handle.into_bytes()
                );
            } else {
                panic!("unexpected event kind: expected NewSubcription event")
            }
        }

        fn assert_cancelled_subscription(event: &EmittedEvent, expected_for_account: AccountId) {
            let decoded_event = <Event as scale::Decode>::decode(&mut &event.data[..])
                .expect("invalid event buffer");
            if let Event::CancelledSubscription(CancelledSubscription { for_account }) =
                decoded_event
            {
                assert_eq!(for_account, expected_for_account);
            } else {
                panic!("unexpected event kind: expected CancelSubscription event")
            }
        }

        fn assert_cancelled_subscriptions(
            event: &EmittedEvent,
            expected_for_accounts: Vec<AccountId>,
        ) {
            let decoded_event = <Event as scale::Decode>::decode(&mut &event.data[..])
                .expect("invalid event buffer");
            if let Event::CancelledSubscriptions(CancelledSubscriptions { for_accounts }) =
                decoded_event
            {
                assert_eq!(for_accounts, expected_for_accounts);
            } else {
                panic!("unexpected event kind: expected CancelSubscriptions event")
            }
        }
    }
}
