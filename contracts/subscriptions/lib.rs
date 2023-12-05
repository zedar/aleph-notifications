#![cfg_attr(not(feature = "std"), no_std, no_main)]

// Notes:
// Block time (Aleph Zero): The average time it takes the network to generate a new block. The block time of Aleph Zero is set to 1 second.

#[ink::contract]
mod subscriptions {

    use ink::storage::Mapping;

    /// Defines subscription payment interval
    #[derive(Debug, Clone, PartialEq, scale::Encode, scale::Decode)]
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
        /// Number of remaining intervalas
        intervals_to_pay: u32,
        /// Number of already paid intervals
        paid_intervals: u32,
        /// Price per interval calculated at the time of subscription registration
        /// Units - the smallest unit, e.g. 1_000_000_000_000 = 1DZERO, 1TZERO, 1AZERO
        price_per_interval: u128,
        /// Registered at
        registered_at: BlockNumber,
        /// Last payment at
        last_payment: BlockNumber,
        /// External channel handle specific for the subscription, e.g. Telegram channel ID
        external_channel_handle: String,
    }
    /// Defines the storage layout of this smart contract.
    #[ink(storage)]
    pub struct Subscriptions {
        /// Only owner of this smart contract can start payment settlements and can transfer ownership
        owner: AccountId,
        /// Price per subscription per block that can be translated to a payment interval
        /// Units - the smallest unit, e.g. 1_000_000_000_000 = 1DZERO, 1TZERO, 1AZERO
        price_per_block: u128,
        /// Registered and active subscriptions
        subscriptions: Mapping<AccountId, Subscription>,
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
        SubscriptionCostTooHigh(u128),
        /// Ink! error can be converted to this smart contract errors
        InkEnvFailure(String),
    }

    /// Converts ink::env::Error to this smart contract error
    impl From<ink::env::Error> for Error {
        fn from(value: ink::env::Error) -> Self {
            Error::InkEnvFailure(format!("{:?}", value))
        }
    }

    /// Event emitted when a new subscription has been added
    #[ink(event)]
    pub struct NewSubscription {
        /// Who registerred new subscription. Events published for this account will result in notifications
        #[ink(topic)]
        for_account: AccountId,

        /// A handle (e.g. chat_id) associated with the user's subscription
        external_channel_handle: Vec<u8>,
    }

    impl Subscriptions {
        /// Creates new instance of this smart contract with empty list of subscriptions.
        /// The caller of this function becomes an owner of the subscriptions registry.
        /// Only the owner can start payment settlement and transfer an ownership.
        /// Parameters:
        /// * `price_per_block` - price the subscriber needs to pay for the number of blocks translated to the payment interval.
        #[ink(constructor)]
        pub fn new(price_per_block: u128) -> Self {
            Self {
                owner: Self::env().caller(),
                price_per_block,
                subscriptions: Mapping::default(),
            }
        }

        /// Registers new subscrption for a caller and a given time period.
        /// Parameters:
        /// * payment_interval - one of week|month
        /// * intervals_to_pay - number of paid intervales declared by the caller
        /// * external_channel_handle_id - external identifier, specific for the external channel, used by the notification service
        /// * external_channel - external channel as identified by the string, e.g. telegram
        /// Errors:
        #[ink(message)]
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
            let price_per_interval = self.calc_price_per_interval(&payment_interval);
            let subscription = Subscription {
                payment_interval,
                intervals_to_pay,
                paid_intervals: 1,
                price_per_interval,
                registered_at: curr_block,
                last_payment: curr_block,
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

            self.env().emit_event(NewSubscription {
                for_account: caller,
                external_channel_handle: external_channel_handle.into_bytes(),
            });

            Ok(())
        }

        // fn authorized(&self, caller: AccountId) -> Result<(), Error> {
        //     if caller != self.owner {
        //         return Err(Error::NotAuthorized);
        //     }
        //     Ok(())
        // }

        fn calc_price_per_interval(&self, payment_interval: &PaymentInterval) -> u128 {
            self.price_per_block
                * match payment_interval {
                    PaymentInterval::Week => 604800u128,   // 7*24*3600s
                    PaymentInterval::Month => 2592000u128, //30*24*3600s
                }
        }

        /// Transfers amount of tokens from the contract's account to the owner account.
        fn transfer_to_owner(&self, amount: u128) {
            if Self::env().transfer(self.owner, amount).is_err() {
                panic!("failed to transfer tokens to owner")
            }
        }

        /// Reimburses the caller with overpaid tokens.
        /// Panics if the transfer fails - this means this contract's balance is
        /// too low which means something went wrong.
        fn reimburse(&self, recipient: AccountId, amount: u128) {
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

        fn assert_new_subscription(
            event: &EmittedEvent,
            expected_for_account: AccountId,
            expected_external_channel_handle: String,
        ) {
            let decoded_event = <Event as scale::Decode>::decode(&mut &event.data[..])
                .expect("invalid event buffer");
            let Event::NewSubscription(NewSubscription {
                for_account,
                external_channel_handle,
            }) = decoded_event;
            assert_eq!(for_account, expected_for_account);
            assert_eq!(
                external_channel_handle,
                expected_external_channel_handle.into_bytes()
            );
        }
    }
}
