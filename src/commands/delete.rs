use std::path::Path;

use crate::client::error::ClientError;
use crate::commands::extract_str;
use crate::database::postgresql::{PgPool, PgPooled};
use crate::database::schemas::servers::dsl as servers_dsl;
use crate::util::data_types::ServNames;
use crate::util::{EMBED_COLOR, get_pool_from_ctx};
use diesel::OptionalExtension;
use diesel::{ExpressionMethods, QueryDsl, delete};
use diesel_async::RunQueryDsl;
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateEmbed, Permissions,
};
use tokio::fs;
use tokio::process::Command;

pub async fn run(ctx: &Context, command: &CommandInteraction) -> Result<(), ClientError> {
    let name = extract_str("name", &command.data.options())?.to_lowercase();

    let pool: PgPool = get_pool_from_ctx(ctx).await?;
    let mut conn: PgPooled = pool.get().await?;

    let server: Option<(i64, i64, bool)> = servers_dsl::servers
        .filter(servers_dsl::name.eq(&name))
        .select((servers_dsl::id, servers_dsl::creator, servers_dsl::started))
        .first::<(i64, i64, bool)>(&mut conn)
        .await
        .optional()?;

    if let Some((id, creator, started)) = server {
        if command.user.id.get().cast_signed() != creator
            && !command
                .member
                .clone()
                .unwrap()
                .permissions
                .unwrap()
                .contains(Permissions::MANAGE_GUILD)
        {
            return Err(ClientError::OtherStatic(
                "Vous n'avez pas la permission pour suprimer le serveur.",
            ));
        }

        if started {
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

        let r = Command::new("docker")
            .args(["compose", "rm", "-f"])
            .current_dir(Path::new("worlds").join(id.to_string()))
            .status()
            .await?;

        if !r.success() {
            return Err(ClientError::Other(
                "Erreur au démarrage du serv".to_string(),
            ));
        }

        fs::remove_dir_all(Path::new("worlds").join(id.to_string())).await?;

        delete(servers_dsl::servers)
            .filter(servers_dsl::name.eq(&name))
            .execute(&mut pool.get().await?)
            .await?;

        {
            let mut data = ctx.data.write().await;

            if let Some(strings) = data.get_mut::<ServNames>() {
                strings.retain(|s| *s != name);
            }
        }

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
    } else {
        Err(ClientError::OtherStatic("Ce serveur n'existe pas."))
    }
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
            .max_length(25)
            .set_autocomplete(true),
        )
}
