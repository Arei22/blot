pub mod data;
pub mod error;
mod serenity_handler;

use crate::client::data::PgPoolData;
use crate::client::serenity_handler::SerenityHandler;
use crate::database::postgresql::{PgPooled, get_pool};
use crate::database::schemas::servers::dsl as servers_dsl;
use crate::util::data_types::{ServNames, VersionsList};
use crate::util::parse_key;
use diesel::QueryDsl;
use diesel_async::RunQueryDsl;
use serenity::prelude::GatewayIntents;
use std::error::Error;
use tokio::fs;

pub struct Client {
    client: serenity::Client,
}

impl Client {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let intents: GatewayIntents = GatewayIntents::GUILD_MEMBERS
            | GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT;
        let client: serenity::Client =
            serenity::Client::builder(parse_key::<String>("DISCORD_TOKEN")?, intents)
                .event_handler(SerenityHandler)
                .application_id(parse_key::<u64>("DISCORD_APP_ID")?.into())
                .await?;

        let pool = get_pool().await;

        let json = fs::read_to_string("versions.json").await?;
        let versions: Vec<String> = serde_json::from_str(&json)?;

        let pool2 = pool.clone();
        let mut conn: PgPooled = pool2.get().await?;

        let names: Vec<String> = servers_dsl::servers
            .select(servers_dsl::name)
            .order_by(servers_dsl::name)
            .load::<String>(&mut conn)
            .await?;

        {
            let mut data = client.data.write().await;
            data.insert::<PgPoolData>(pool);
            data.insert::<VersionsList>(versions);
            data.insert::<ServNames>(names);
        }

        Ok(Self { client })
    }

    #[inline]
    pub async fn start(&mut self) -> Result<(), serenity::Error> {
        self.client.start().await
    }
}
