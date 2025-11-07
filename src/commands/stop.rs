use std::path::Path;

use crate::client::error::ClientError;
use crate::commands::extract_str;
use crate::database::postgresql::{PgPool, PgPooled};
use crate::database::schemas::servers::dsl as servers_dsl;
use crate::util::{EMBED_COLOR, get_pool_from_ctx};
use diesel::dsl::exists;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption, CreateEmbed,
};
use tokio::process::Command;

pub async fn run(ctx: &Context, command: &CommandInteraction) -> Result<(), ClientError> {
    let name = extract_str("name", &command.data.options())?.to_lowercase();

    let pool: PgPool = get_pool_from_ctx(ctx).await?;
    let mut conn: PgPooled = pool.get().await?;

    let serv_exist: bool = diesel::select(exists(
        servers_dsl::servers.filter(servers_dsl::name.eq(&name)),
    ))
    .get_result(&mut conn)
    .await?;

    if !serv_exist {
        return Err(ClientError::OtherStatic("Ce serveur n'existe pas."));
    }

    let serv_started: bool = diesel::select(exists(
        servers_dsl::servers
            .filter(servers_dsl::started.eq(true))
            .filter(servers_dsl::name.eq(&name)),
    ))
    .get_result(&mut conn)
    .await?;

    if !serv_started {
        return Err(ClientError::OtherStatic("Le serveur n'est pas lancé."));
    }

    let embed = CreateEmbed::new()
        .description(format!("**Arrêt du serveur ``{name}`` en cours...**"))
        .color(EMBED_COLOR);

    command
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new().add_embed(embed),
        )
        .await?;

    let id: i64 = servers_dsl::servers
        .select(servers_dsl::id)
        .filter(servers_dsl::name.eq(&name))
        .get_result(&mut conn)
        .await?;

    Command::new("docker")
        .args(["compose", "down"])
        .current_dir(Path::new("worlds").join(id.to_string()))
        .status()
        .await?;

    diesel::update(servers_dsl::servers.filter(servers_dsl::name.eq(&name)))
        .set(servers_dsl::started.eq(false))
        .execute(&mut conn)
        .await?;

    log::info!("server stoped : {name}!");

    let edited_embed = CreateEmbed::new()
        .description(format!("**Serveur ``{name}`` arrêté !**"))
        .color(EMBED_COLOR);

    command
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new().add_embed(edited_embed),
        )
        .await?;

    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("stop")
        .description("Stop a server.")
        .description_localized("en-US", "Stop a server.")
        .description_localized("en-GB", "Stop a server.")
        .description_localized("fr", "Arrête un serveur.")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "name",
                "Le nom du serveur a arrêté.",
            )
            .description_localized("en-US", "The name of the server to stop.")
            .description_localized("en-GB", "The name of the server to stop.")
            .required(true)
            .max_length(25),
        )
}
