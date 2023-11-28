mod cli;
mod events;

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
        cli::Commands::RewardEvent { for_account } => {
            events.log_reward_events(conn, for_account).await?
        }
    }

    Ok(())
}
