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
            cli::Targets::Telegram { token, chat_id } => {
                let telegram_bot =
                    Arc::new(notifications::telegram::TelegramBot::new(token, chat_id)?);
                events
                    .send_transfer_event_notification(conn, to_account, telegram_bot)
                    .await?
            }
        },
    }

    Ok(())
}
