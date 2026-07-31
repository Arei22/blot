use crate::client::error::ClientError;
use crate::commands::extract_str;
use crate::database::postgresql::{PgPool, PgPooled};
use crate::database::schemas::servers::dsl as servers_dsl;
use crate::util::{EMBED_COLOR, get_pool_from_ctx, parse_key};
use diesel::{ExpressionMethods, QueryDsl, Queryable};
use diesel_async::RunQueryDsl;
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption, UserId,
};
use serenity::builder::CreateEmbed;
use serenity::prelude::Mentionable;

#[derive(Debug, Clone, Queryable)]
struct Server {
    pub name: String,
    pub version: String,
    pub crack: bool,
    pub port: i64,
    pub creator: i64,
    pub moded: bool,
    pub started: bool,
}

async fn get_server(ctx: &Context, name: String) -> Result<Server, ClientError> {
    let pool: PgPool = get_pool_from_ctx(ctx).await?;
    let mut conn: PgPooled = pool.get().await?;

    let server = servers_dsl::servers
        .filter(servers_dsl::name.eq(&name))
        .select((
            servers_dsl::name,
            servers_dsl::version,
            servers_dsl::crack,
            servers_dsl::port,
            servers_dsl::creator,
            servers_dsl::moded,
            servers_dsl::started,
        ))
        .get_result::<Server>(&mut conn)
        .await?;

    Ok(server)
}

pub async fn run(ctx: &Context, command: &CommandInteraction) -> Result<(), ClientError> {
    let name = extract_str("name", &command.data.options())?.to_lowercase();

    let server = get_server(ctx, name).await?;

    let ip = parse_key::<String>("IP")?;

    let server_string: String = format!(
        "* **{}**\n  * **Adresse** : ``{}:{}``\n  * **Version** : ``{}``\n  * **Crack** : ``{}``\n  * **moddé** : ``{}``\n  * **créateur** : {}\n  * **Démarré** : ``{}``",
        server.name,
        ip,
        server.port,
        server.version,
        if server.crack { "oui" } else { "non" },
        if server.moded { "oui" } else { "non" },
        UserId::new(server.creator as u64).mention(),
        if server.started { "oui" } else { "non" },
    );

    let embed = CreateEmbed::new()
        .title("Liste des serveurs")
        .description(server_string)
        .color(EMBED_COLOR);

    command
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new().add_embed(embed),
        )
        .await?;

    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("info")
        .description("Lists all informations about a server.")
        .description_localized("en-US", "Lists all informations about a server.")
        .description_localized("en-GB", "Lists all informations about a server.")
        .description_localized("fr", "Liste tous les informations d'un serveur.")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "name",
                "Le nom du serveur a supprimer.",
            )
            .description_localized("en-US", "The name of the server to delete.")
            .description_localized("en-GB", "The name of the server to delete.")
            .required(true)
            .max_length(25)
            .set_autocomplete(true),
        )
}
