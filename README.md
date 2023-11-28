Aleph zero notification service
===============================

`notification_service` is `Aleph Zero` network client that subscribes to events and sends notification to configurable channels (e.g. Telegram).

# Setup

## Prerequisites

### `rustup` installation 

	$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
	$ source ~/.cargo/env

## Building and testing

## Usage scenarios

	$ cargo run --release -- -n wss://ws.test.azero.dev:443 reward-event -a 5GRkePp3CqPJXkfbt52G4XdZp4Pi4oQaHyERNrFX6fxB73HU
