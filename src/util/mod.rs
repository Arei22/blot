use crate::client::data::PgPoolData;
use crate::client::error::ClientError;
use crate::database::postgresql::PgPool;
use serenity::all::Context;
use std::str::FromStr;
use tokio::signal::unix::SignalKind;

pub mod fileshare;
pub mod logger;

pub const EMBED_COLOR: u64 = 14423107;

#[inline]
pub async fn wait_for_shutdown_signal() {
    let sigint = tokio::signal::ctrl_c();
    let mut sigterm = match tokio::signal::unix::signal(SignalKind::terminate()) {
        Ok(sigterm) => sigterm,
        Err(error) => {
            log::error!("Unable to listen for SIGTERM: {error}. Bot shutdown...");
            std::process::exit(1);
        }
    };

    tokio::select! {
        _ = sigint => log::warn!("Server received SIGINT..."),
        _ = sigterm.recv() => log::warn!("Server received SIGTERM..."),
    }
}

#[inline]
pub fn exit_if_key_not_exist(key: &str) {
    if std::env::var(key).is_err() {
        log::error!("The key {key} does not exist in the .env file.");
        std::process::exit(1);
    }
}

#[inline]
pub fn exit_if_keys_not_exist(keys: &[&str]) {
    for key in keys {
        exit_if_key_not_exist(key);
    }
}

pub fn parse_key<T: FromStr>(key: &str) -> Result<T, ClientError>
where
    <T as FromStr>::Err: std::fmt::Display,
{
    std::env::var(key)?.parse::<T>().map_err(|error| {
        ClientError::Other(format!(
            "Failed to parse `{key}` as `{}`: {error}",
            std::any::type_name::<T>(),
        ))
    })
}

pub fn get_time() -> Result<u64, ClientError> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| {
            ClientError::Other(format!(
                "An error occurred while retrieving the UnixTime: {error}"
            ))
        })?
        .as_secs())
}

pub async fn get_pool_from_ctx(ctx: &Context) -> Result<PgPool, ClientError> {
    ctx.data
        .read()
        .await
        .get::<PgPoolData>()
        .ok_or(ClientError::OtherStatic("Missing PgPoolData in data."))
        .cloned()
}
