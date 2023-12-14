Aleph zero notification service
===============================

`notification_service` is `Aleph Zero` network client that subscribes to events and sends notification to variaty of channels (e.g. Telegram).

# Repository structure

This repository contains:

* `cli.rs` - command line application interface. Use `-h` option for the list of available commands.
* `events.rs` - aleph node event subscriber
* `notifications` - event notification channels, e.g. Telegram
* `subscriptions` - aleph node Subscriptions smart contract client, listening for events e.g. `NewSubscription`, `CancelledSubscription`, `CancelledSubscriptions`
* `Makefile` - helper commands used to build and test application. Use `make help` for the list of available commands

# Setup

## Prerequisites

### `rustup` installation 

	$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
	$ source ~/.cargo/env

## Building and testing

The application is built using the `--release` flag to minimize the size of the binary.

	$ make build-service

## Telegram bot configuration

Telegram requires bot to be configured for using Telegram REST API. In order to create a new bot use [@Botfather](https://t.me/botfather) the the format `123456789:aaaabbbbcccc`.

Find the Telegram `chat_id` or `group_chat_id` where notifications will be sent. In a future version, this feature will be dynamically configured as a part of subscription procedure.
Use [@RawDataBot](https://telegram.me/rawdatabot) to get your `chat_id`, e.g. `222222222` from the json message below (`message/chat/id`). 
```json
    "update_id": 111111111,
    "message": {
        "chat": {
            "id": 222222222,
            "first_name": "user.azero",
            "username": "username",
            "type": "private"
        },
    }
}
```
Find the `aleph-notifications` Telegram bot and start the channel. This step is required by the bot to send messages to your `chat_id`.

## Usage scenarios

### Send Transfer event notification to the Telegram channel

Send notifications about transfer event to the Telegram channel. Connect to the local node (version `r-12.1`).

  $ ./notification-service/target/release/notification_service -n ws://127.0.0.1:9944 -c <smart contract address> -m contracts/subscriptions/target/ink/subscriptions.json transfer-event telegram --token <telegram bot token>

