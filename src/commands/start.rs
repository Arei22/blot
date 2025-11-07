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
        servers_dsl::servers.filter(servers_dsl::started.eq(true)),
    ))
    .get_result(&mut conn)
    .await?;

    if serv_started {
        return Err(ClientError::OtherStatic("Un serveur est déjà lancé."));
    }

    let embed = CreateEmbed::new()
        .description(format!("**Démarrage du serveur ``{name}`` ...**"))
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

    let r = Command::new("docker")
        .args(["compose", "up", "-d", "--wait"])
        .current_dir(Path::new("worlds").join(id.to_string()))
        .status()
        .await?;

    if !r.success() {
        return Err(ClientError::Other(
            "Erreur au démarrage du serv".to_string(),
        ));
    }

    diesel::update(servers_dsl::servers.filter(servers_dsl::name.eq(&name)))
        .set(servers_dsl::started.eq(true))
        .execute(&mut conn)
        .await?;

    log::info!("server started : {name}!");

    let edited_embed = CreateEmbed::new()
        .description(format!("**Serveur ``{name}`` démaré !**"))
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
    CreateCommand::new("start")
        .description("Start a server.")
        .description_localized("en-US", "start a server.")
        .description_localized("en-GB", "start a server.")
        .description_localized("fr", "Démarer un serveur.")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "name",
                "Le nom du serveur a demarer.",
            )
            .description_localized("en-US", "The name of the server to start.")
            .description_localized("en-GB", "The name of the server to start.")
            .required(true)
            .max_length(25),
        )
}
