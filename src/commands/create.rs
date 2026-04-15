use std::path::Path;
use std::time::Duration;

use crate::client::error::ClientError;
use crate::commands::extract_bool_optional;
use crate::commands::extract_str;
use crate::commands::extract_str_optional;
use crate::database::postgresql::PgPool;
use crate::database::postgresql::PgPooled;
use crate::database::schemas::servers::dsl as servers_dsl;
use crate::util::fileshare::generate_upload;
use crate::util::fileshare::get_upload;
use crate::util::parse_key;
use crate::util::{EMBED_COLOR, get_pool_from_ctx};
use diesel::dsl::exists;
use diesel::{ExpressionMethods, QueryDsl, insert_into};
use diesel_async::RunQueryDsl;
use serde_yml::Mapping;
use serde_yml::Value;
use serenity::all::CommandInteraction;
use serenity::all::MessageCollector;
use serenity::all::{CommandOptionType, Context, CreateCommand, CreateCommandOption, CreateEmbed};
use tokio::fs;

pub async fn run(ctx: &Context, command: &CommandInteraction) -> Result<(), ClientError> {
    let name = extract_str("name", &command.data.options())?.to_lowercase();
    let ver = extract_str_optional("version", &command.data.options())?;
    let difficulty_option = extract_str_optional("difficulty", &command.data.options())?;
    let map = extract_bool_optional("map", &command.data.options())?.unwrap_or(false);
    let modpack = extract_str_optional("modpack", &command.data.options())?;

    let pool: PgPool = get_pool_from_ctx(ctx).await?;
    let mut conn: PgPooled = pool.get().await?;

    let serv_exist: bool = diesel::select(exists(
        servers_dsl::servers.filter(servers_dsl::name.eq(&name)),
    ))
    .get_result(&mut conn)
    .await?;
    if serv_exist {
        return Err(ClientError::OtherStatic("Ce nom de serveur existe déjà."));
    }

    let ports_used: Vec<i64> = servers_dsl::servers
        .select(servers_dsl::port)
        .order_by(servers_dsl::port)
        .load::<i64>(&mut conn)
        .await?;

    let mut port = 0;

    for i in parse_key::<i64>("MIN_PORT")?..parse_key::<i64>("MAX_PORT")? {
        if !ports_used.contains(&i) {
            port = i;
            break;
        }
    }

    if port == 0 {
        return Err(ClientError::OtherStatic("Pas de port disponible."));
    }

    command
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new().add_embed(
                CreateEmbed::new()
                    .description("**Création d'un serveur.**")
                    .color(EMBED_COLOR),
            ),
        )
        .await?;

    let mut services = Mapping::new();

    let mut mc = Mapping::new();
    mc.insert(
        Value::String("image".into()),
        Value::String("itzg/minecraft-server".into()),
    );
    mc.insert(Value::String("tty".into()), Value::Bool(true));
    mc.insert(Value::String("stdin_open".into()), Value::Bool(true));
    mc.insert(
        Value::String("ports".into()),
        Value::Sequence(vec![Value::String(format!("{port}:25565"))]),
    );

    let mut env = Mapping::new();

    env.insert(Value::String("EULA".into()), Value::String("TRUE".into()));

    env.insert(
        Value::String("OPS".into()),
        Value::String(parse_key::<String>("ADMIN_PLAYER")?),
    );

    if let Some(version) = ver {
        let json = fs::read_to_string("versions.json").await?;
        let versions: Vec<String> = serde_json::from_str(&json)?;

        if versions.iter().any(|e| e == version) {
            env.insert(
                Value::String("VERSION".into()),
                Value::String(version.into()),
            );
        } else {
            return Err(ClientError::Other(format!(
                "{version} n'est pas une version valide."
            )));
        }
    }

    if let Some(difficulty) = difficulty_option {
        env.insert(
            Value::String("DIFFICULTY".into()),
            Value::String(difficulty.into()),
        );
    }

    env.insert(
        Value::String("MAX_MEMORY".into()),
        Value::String(parse_key::<String>("MAX_MEMORY")?),
    );

    let id: i64 = insert_into(servers_dsl::servers)
        .values((
            servers_dsl::name.eq(&name),
            servers_dsl::version.eq(ver.map_or_else(|| "latest", |version| version).to_string()),
            servers_dsl::difficulty
                .eq(difficulty_option.map_or_else(|| "easy", |difficulty| difficulty)),
            servers_dsl::port.eq(port),
            servers_dsl::started.eq(false),
        ))
        .returning(servers_dsl::id)
        .get_result(&mut conn)
        .await?;

    mc.insert(
        Value::String("volumes".into()),
        Value::Sequence(vec![
            Value::String(format!("/worlds/{id}/data:/data")),
            Value::String(format!("/worlds/{id}/world:/world")),
        ]),
    );

    let mut healthcheck = Mapping::new();

    healthcheck.insert(
        Value::String("test".into()),
        Value::String("mc-health".into()),
    );
    healthcheck.insert(
        Value::String("start_period".into()),
        Value::String("1m".into()),
    );
    healthcheck.insert(Value::String("interval".into()), Value::String("5s".into()));
    healthcheck.insert(Value::String("retries".into()), Value::String("20".into()));

    mc.insert("healthcheck".into(), Value::Mapping(healthcheck));

    let dir = Path::new("worlds").join(id.to_string());

    fs::create_dir_all(dir.join("world")).await?;

    if map {
        let uuid = generate_upload().await?;

        let doup = parse_key::<String>("DOUP_URL")?;

        let embed = CreateEmbed::new()
            .description(format!(
                "**Veuillez upload la map à {doup}/upload?uuid={uuid}**"
            ))
            .color(EMBED_COLOR);

        command
            .edit_response(
                &ctx.http,
                serenity::builder::EditInteractionResponse::new().add_embed(embed),
            )
            .await?;

        get_upload(uuid.clone(), id).await?;

        env.insert(
            Value::String("WORLD".into()),
            Value::String(format!("/world/{uuid}")),
        );
    }

    if let Some(mp) = modpack {
        env.insert(
            Value::String("MODPACK_PLATFORM".into()),
            Value::String("AUTO_CURSEFORGE".into()),
        );
        env.insert(
            Value::String("CF_API_KEY".into()),
            Value::String(parse_key("CF_API_KEY")?),
        );
        if mp == "cf" {
            let embed = CreateEmbed::new()
                .description(format!("**Veuillez écrire le lien du modpack**"))
                .color(EMBED_COLOR);

            command
                .edit_response(
                    &ctx.http,
                    serenity::builder::EditInteractionResponse::new().add_embed(embed),
                )
                .await?;

            let response = MessageCollector::new(ctx)
                .author_id(command.user.id)
                .channel_id(command.channel_id)
                .timeout(Duration::from_secs(60))
                .await;

            if let Some(url) = response {
                env.insert(
                    Value::String("CF_PAGE_URL".into()),
                    Value::String(url.content.clone()),
                );

                url.delete(&ctx.http).await?;
            } else {
                return Err(ClientError::Other(format!(
                    "Vous n'avez pas écris de lien."
                )));
            }
        } else {
            let uuid = generate_upload().await?;

            let doup = parse_key::<String>("DOUP_URL")?;

            let embed = CreateEmbed::new()
                .description(format!(
                    "**Veuillez upload le modpack à {doup}/upload?uuid={uuid}**"
                ))
                .color(EMBED_COLOR);

            command
                .edit_response(
                    &ctx.http,
                    serenity::builder::EditInteractionResponse::new().add_embed(embed),
                )
                .await?;

            get_upload(uuid.clone(), id).await?;

            env.insert(
                Value::String("CF_SLUG".into()),
                Value::String("custom".into()),
            );

            env.insert(
                Value::String("CF_MODPACK_ZIP".into()),
                Value::String(format!("/world/{uuid}")),
            );
        }
    }

    mc.insert(Value::String("environment".into()), Value::Mapping(env));

    services.insert(Value::String("mc".into()), Value::Mapping(mc));

    let mut root = Mapping::new();
    root.insert(Value::String("services".into()), Value::Mapping(services));

    let yml_str = serde_yml::to_string(&root)?;

    fs::write(dir.join("docker-compose.yml"), yml_str).await?;

    let embed = CreateEmbed::new()
        .description(format!("**Le serveur ``{name}`` a bien été créé !**"))
        .color(EMBED_COLOR);

    command
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new().add_embed(embed),
        )
        .await?;

    log::info!("Created \"{name}\" server!");

    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("create")
        .description("Create a server.")
        .description_localized("en-US", "Create a server.")
        .description_localized("en-GB", "Create a server.")
        .description_localized("fr", "Création d'un serveur.")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "name",
                "Le nom du serveur à créer.",
            )
            .description_localized("en-US", "The name of the server to be created.")
            .description_localized("en-GB", "The name of the server to be created.")
            .required(true)
            .max_length(25),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "version",
                "La version du serveur.",
            )
            .description_localized("en-US", "The version of the server to be created.")
            .description_localized("en-GB", "The version of the server to be created.")
            .set_autocomplete(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "difficulty",
                "La difficulté du serveur.",
            )
            .description_localized("en-US", "The difficulty of the server to be created.")
            .description_localized("en-GB", "The difficulty of the server to be created.")
            .add_string_choice("peaceful", "peaceful")
            .add_string_choice("easy", "easy")
            .add_string_choice("normal", "normal")
            .add_string_choice("hard", "hard"),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::Boolean,
            "map",
            "Mettre une map ?",
        ))
        .description_localized("en-US", "Add a map?")
        .description_localized("en-GB", "Add a map?")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "modpack", "Mettre un modpack ?")
                .add_string_choice("Curseforge_URL", "cf")
                .add_string_choice("File", "file"),
        )
        .description_localized("en-US", "Add a modpack?")
        .description_localized("en-GB", "Add a modpack?")
}
