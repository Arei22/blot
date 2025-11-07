use std::path::Path;

use crate::client::error::ClientError;
use crate::commands::extract_str;
use crate::database::postgresql::{PgPool, PgPooled};
use crate::database::schemas::servers::dsl as servers_dsl;
use crate::util::{EMBED_COLOR, get_pool_from_ctx};
use diesel::dsl::exists;
use diesel::{ExpressionMethods, QueryDsl, delete};
use diesel_async::RunQueryDsl;
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption, CreateEmbed,
};
use tokio::fs;

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

    if serv_started {
        return Err(ClientError::OtherStatic("Le serveur est lancé."));
    }

    let embed = CreateEmbed::new()
        .description("**Supression du serveur...**".to_string())
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

    fs::remove_dir_all(Path::new("worlds").join(id.to_string())).await?;

    delete(servers_dsl::servers)
        .filter(servers_dsl::name.eq(&name))
        .execute(&mut pool.get().await?)
        .await?;

    log::info!("Deleted server : {name}!");

    let embed2 = CreateEmbed::new()
        .description(format!("**Serveur ``{name}`` supprimé !**"))
        .color(EMBED_COLOR);

    command
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new().add_embed(embed2),
        )
        .await?;

    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("delete")
        .description("Delete a server.")
        .description_localized("en-US", "Delete a server.")
        .description_localized("en-GB", "Delete a server.")
        .description_localized("fr", "Suppression d'un serveur.")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "name",
                "Le nom du serveur a supprimer.",
            )
            .description_localized("en-US", "The name of the server to delete.")
            .description_localized("en-GB", "The name of the server to delete.")
            .required(true)
            .max_length(25),
        )
}
