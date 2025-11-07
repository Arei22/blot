use blot::client::Client;
use blot::database::postgresql::run_migration;
use blot::util::logger::init as init_logger;
use blot::util::{exit_if_keys_not_exist, wait_for_shutdown_signal};
use std::error::Error;
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init_logger();

    dotenvy::dotenv().ok();

    exit_if_keys_not_exist(&[
        "DISCORD_TOKEN",
        "DISCORD_APP_ID",
        "DISCORD_GUILD_ID",
        "DATABASE_URL",
        "ADMIN_PLAYER",
    ]);

    log::info!("Start blot...");

    run_migration().await;

    let mut bot_client: Client = Client::new().await?;
    let bot_task: JoinHandle<()> = tokio::spawn(async move {
        if let Err(error) = bot_client.start().await {
            log::error!("An error occurred during the initialization of the bot: {error}",);
            std::process::exit(1);
        }
    });

    wait_for_shutdown_signal().await;

    bot_task.abort();
    log::warn!("Program stopped by the user.");

    Ok(())
}
