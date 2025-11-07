use crate::client::error::ClientError;
use crate::database::postgresql::{PgPool, PgPooled};
use crate::database::schemas::servers::dsl as servers_dsl;
use crate::util::{EMBED_COLOR, get_pool_from_ctx, parse_key};
use diesel::{QueryDsl, Queryable};
use diesel_async::RunQueryDsl;
use serenity::all::{
    CommandInteraction, Context, CreateActionRow, CreateButton, CreateCommand, CreateEmbedFooter,
    EditInteractionResponse,
};
use serenity::builder::CreateEmbed;

const ELEMENT_PER_PAGE: u64 = 4;

#[derive(Debug, Clone, Queryable)]
struct ServersList {
    pub name: String,
    pub version: String,
    pub difficulty: String,
    pub port: i64,
    pub started: bool,
}

async fn get_servers(
    ctx: &Context,
    page: &mut u64,
) -> Result<(Vec<ServersList>, u64), ClientError> {
    let pool: PgPool = get_pool_from_ctx(ctx).await?;
    let mut conn: PgPooled = pool.get().await?;

    let servers_count = servers_dsl::servers
        .count()
        .get_result::<i64>(&mut conn)
        .await?;

    if *page != 0 {
        *page = (*page).min((servers_count as u64).div_ceil(ELEMENT_PER_PAGE));
    }

    #[allow(clippy::cast_possible_wrap)]
    let servers: Vec<ServersList> = servers_dsl::servers
        .select((
            servers_dsl::name,
            servers_dsl::version,
            servers_dsl::difficulty,
            servers_dsl::port,
            servers_dsl::started,
        ))
        .limit(ELEMENT_PER_PAGE as i64)
        .offset(i64::try_from(page.saturating_sub(1) * ELEMENT_PER_PAGE)?)
        .order_by(servers_dsl::name)
        .load::<ServersList>(&mut conn)
        .await?;

    Ok((servers, (servers_count as u64).div_ceil(ELEMENT_PER_PAGE)))
}

pub async fn run(ctx: &Context, command: &CommandInteraction) -> Result<(), ClientError> {
    let (servers, pages_count) = get_servers(ctx, &mut 0).await?;
    if servers.is_empty() {
        let embed = CreateEmbed::new()
            .description("Aucun serveur n'a été créé.")
            .color(EMBED_COLOR);

        command
            .edit_response(
                &ctx.http,
                serenity::builder::EditInteractionResponse::new().add_embed(embed),
            )
            .await?;

        return Ok(());
    }

    let ip = parse_key::<String>("IP")?;

    let servers_strings: Vec<String> = servers.iter().map(|server| format!(
            "* **{}**\n  * **Adresse** : ``{}:{}``\n  * **Version** : ``{}``\n  * **Difficulté** : ``{}``\n  * **Démarré** : ``{}``",
            server.name,
            ip,
            server.port,
            server.version,
            server.difficulty,
            if server.started {"oui"} else {"non"},
        )).collect();

    let embed = CreateEmbed::new()
        .title("Liste des serveurs")
        .description(servers_strings.join("\n"))
        .footer(CreateEmbedFooter::new(format!("Page 1/{pages_count}")))
        .color(EMBED_COLOR);

    command
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new()
                .add_embed(embed)
                .button(
                    CreateButton::new("page-0")
                        .label("Précédent")
                        .disabled(true),
                )
                .button(
                    CreateButton::new("page-2")
                        .label("Suivant")
                        .disabled(pages_count == 1),
                ),
        )
        .await?;

    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("list")
        .description("Lists all created servers.")
        .description_localized("en-US", "Lists all created servers.")
        .description_localized("en-GB", "Lists all created servers.")
        .description_localized("fr", "Liste tous les serveurs créés.")
}

pub async fn get_page(
    ctx: &Context,
    mut page: u64,
) -> Result<EditInteractionResponse, ClientError> {
    page = page.max(1);
    let (servers, pages_count) = get_servers(ctx, &mut page).await?;
    if servers.is_empty() {
        let msg: EditInteractionResponse = EditInteractionResponse::new().embed(
            CreateEmbed::new()
                .description("Aucun serveur n'a été créé.")
                .color(EMBED_COLOR),
        );

        return Ok(msg);
    }

    let ip = parse_key::<String>("IP")?;

    let servers_strings: Vec<String> = servers.iter().map(|server| format!(
            "* **{}**\n  * **Adresse** : ``{}:{}``\n  * **Version** : ``{}``\n  * **Difficulté** : ``{}``\n  * **Démarré** : ``{}``",
            server.name,
            ip,
            server.port,
            server.version,
            server.difficulty,
            if server.started {"oui"} else {"non"},
        )).collect();

    let msg: EditInteractionResponse = EditInteractionResponse::new()
        .embed(
            CreateEmbed::new()
                .title("Liste des serveurs")
                .description(servers_strings.join("\n"))
                .footer(CreateEmbedFooter::new(format!("Page {page}/{pages_count}")))
                .color(EMBED_COLOR),
        )
        .components(vec![CreateActionRow::Buttons(vec![
            CreateButton::new(format!("page-{}", page - 1))
                .label("Précédent")
                .disabled(page == 1),
            CreateButton::new(format!("page-{}", page + 1))
                .label("Suivant")
                .disabled(pages_count == page),
        ])]);

    Ok(msg)
}
