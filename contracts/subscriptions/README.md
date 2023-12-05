Subscription smart contract for the aleph-notification service
==============================================================

`subscriptions` is a contract that allows users to subscribe for aleph zero chain events and get notified via variaty of channels (e.g. Telegram).

# Setup

## Prerequisites

* Rust nightly
* `cargo-contract` compatible with current (`r-12.1`) aleph zero node: `cargo install cargo-contract --version 2.0.1 --force --lock`

# Architecture

The `Subscriptions` smart contract allows to subscribe to on-chain event notifications, e.g. Rewarded nominator event.
In the base version, the subscriber declares the length of the subscriptions periods (e.g. n-weeks), and the contract owner starts payment settlements on regular basis.
The subscriber must provide a token value sufficient to pay for the declared subscription period. The token value is transferred to the smart contract.
When a subscriber cancels subscription, the remaining tokens will be returned to the subscriber's account.
In a future version, the contract will notify the subscriber to accept payment for the next period. Then subscriber will keep their tokens in their wallets.
