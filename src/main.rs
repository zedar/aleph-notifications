#![feature(trait_alias)]

mod cli;
mod events;
mod notifications;

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

    log::info!("Establishing connection...");
    let conn = aleph_client::Connection::new(&cli.node_address).await;
    log::info!("Connection is live...");

    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(SIGINT, Arc::clone(&term))?;

    let events = Events::new(Arc::clone(&term))?;

    match cli.commands {
        cli::Commands::TransferEvent {
            to_account,
            targets,
        } => match targets {
            cli::Targets::Telegram { token, user } => {
                let telegram_bot =
                    notifications::telegram::TelegramBot::new(token, user.try_into()?)?;
                events
                    .send_transfer_event_notification(conn, to_account, &telegram_bot)
                    .await?
            }
        },
        cli::Commands::RewardedEvent {
            for_account,
            targets,
        } => match targets {
            cli::Targets::Telegram { token, user } => {
                let telegram_bot =
                    notifications::telegram::TelegramBot::new(token, user.try_into()?)?;
                events
                    .send_rewarded_event_notification(conn, for_account, &telegram_bot)
                    .await?
            }
        },
    }

    Ok(())
}
