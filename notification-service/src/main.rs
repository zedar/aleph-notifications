#![feature(trait_alias)]

mod cli;
mod events;
mod notifications;
mod subscriptions;

use std::sync::{atomic::AtomicBool, Arc};

use anyhow::Result;
use clap::Parser;
use env_logger::Env;
use events::Events;
use signal_hook::consts::SIGINT;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    env_logger::Builder::from_env(Env::default().default_filter_or(&cli.log_level)).init();

    log::info!("{:?}", cli);

    log::info!("Establishing smart contract client...");
    let mut subscriptions =
        subscriptions::Subscriptions::new(cli.sc_address, &cli.node_address, &cli.sc_metadata)?;
    log::info!("Initializing subscriptions...");
    subscriptions.init_subscriptions().await?;
    log::info!("Smart contract client is live... {:?}", subscriptions);

    log::info!("Establishing connection...");
    let conn = aleph_client::Connection::new(&cli.node_address).await;
    log::info!("Connection is live...");

    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(SIGINT, Arc::clone(&term))?;

    let events = Events::new(
        Arc::clone(&term),
        subscriptions.active_subscriptions.clone(),
    )?;

    match cli.commands {
        cli::Commands::TransferEvent { targets } => match targets {
            cli::Targets::Telegram { token } => {
                let telegram_bot = notifications::telegram::TelegramBot::new(token)?;
                events
                    .send_transfer_event_notification(conn, &telegram_bot)
                    .await?
            }
        },
        cli::Commands::RewardedEvent { targets } => match targets {
            cli::Targets::Telegram { token } => {
                let telegram_bot = notifications::telegram::TelegramBot::new(token)?;
                events
                    .send_rewarded_event_notification(conn, &telegram_bot)
                    .await?
            }
        },
    }

    Ok(())
}
