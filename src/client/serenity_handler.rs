use crate::client::error::ClientError;
use crate::commands;
use crate::interarction::autocomplete_version::autocomplete_version;
use crate::interarction::button_list::button_list;
use crate::util::{EMBED_COLOR, parse_key};
use serenity::all::{CreateEmbed, CreateInteractionResponseMessage};
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{application::Interaction, gateway::Ready, id::GuildId},
};
pub struct SerenityHandler;

#[async_trait]
impl EventHandler for SerenityHandler {
    async fn ready(&self, ctx: Context, _ready: Ready) {
        log::info!("blot is connected!");

        let guild_id = GuildId::new(parse_key::<u64>("DISCORD_GUILD_ID").unwrap());

        let result = guild_id
            .set_commands(
                &ctx.http,
                vec![
                    commands::create::register(),
                    commands::list::register(),
                    commands::delete::register(),
                    commands::start::register(),
                    commands::stop::register(),
                ],
            )
            .await;
        if let Err(error) = result {
            log::error!("Unable to register commands: {error}. Bot shutdown...");
            std::process::exit(1);
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let embed = CreateEmbed::new()
                .description("**Chargement de la commande...**")
                .color(EMBED_COLOR);
            let _ = command
                .create_response(
                    &ctx.http,
                    serenity::builder::CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().add_embed(embed),
                    ),
                )
                .await;
            let res = match command.data.name.as_str() {
                "create" => commands::create::run(&ctx, &command).await,
                "list" => commands::list::run(&ctx, &command).await,
                "delete" => commands::delete::run(&ctx, &command).await,
                "start" => commands::start::run(&ctx, &command).await,
                "stop" => commands::stop::run(&ctx, &command).await,
                _ => Err(ClientError::OtherStatic(
                    "Slash command defined at Discord but not in the bot.",
                )),
            };
            if let Err(error) = res {
                let embed = CreateEmbed::new()
                    .description(error.to_string())
                    .color(EMBED_COLOR);

                if let Err(err) = command
                    .edit_response(
                        &ctx.http,
                        serenity::builder::EditInteractionResponse::new().add_embed(embed),
                    )
                    .await
                {
                    log::error!("Cannot respond to slash command: {err}");
                }
            }
        } else if let Interaction::Component(component) = interaction {
            if component.data.custom_id.starts_with("page-") {
                button_list(ctx, component).await;
            }
        } else if let Interaction::Autocomplete(command) = interaction
            && command.data.name == "create"
            && let Err(e) = autocomplete_version(ctx, command).await
        {
            log::error!("{e}");
        }
    }
}
