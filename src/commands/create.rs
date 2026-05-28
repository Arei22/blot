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
use diesel::delete;
use diesel::dsl::exists;
use diesel::{ExpressionMethods, QueryDsl, insert_into};
use diesel_async::RunQueryDsl;
use serde_yml::Mapping;
use serde_yml::Value;
use serenity::all::CommandInteraction;
use serenity::all::MessageCollector;
use serenity::all::{CommandOptionType, Context, CreateCommand, CreateCommandOption, CreateEmbed};
use tokio::fs;

pub struct Args<'a> {
    name: String,
    ver: Option<&'a str>,
    difficulty_option: Option<&'a str>,
    map: bool,
    modpack: Option<&'a str>,
    crack: bool,
}

pub async fn run(ctx: &Context, command: &CommandInteraction) -> Result<(), ClientError> {
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

    let args = Args {
        name: extract_str("name", &command.data.options())?.to_lowercase(),
        ver: extract_str_optional("version", &command.data.options())?,
        difficulty_option: extract_str_optional("difficulty", &command.data.options())?,
        map: extract_bool_optional("map", &command.data.options())?.unwrap_or(false),
        modpack: extract_str_optional("modpack", &command.data.options())?,
        crack: extract_bool_optional("crack", &command.data.options())?.unwrap_or(false),
    };

    let pool: PgPool = get_pool_from_ctx(ctx).await?;
    let mut conn: PgPooled = pool.get().await?;

    let serv_exist: bool = diesel::select(exists(
        servers_dsl::servers.filter(servers_dsl::name.eq(&args.name)),
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

    let id: i64 = insert_into(servers_dsl::servers)
        .values((
            servers_dsl::name.eq(&args.name),
            servers_dsl::version.eq(args.ver.unwrap_or("latest").to_string()),
            servers_dsl::crack.eq(args.crack),
            servers_dsl::port.eq(port),
            servers_dsl::started.eq(false),
        ))
        .returning(servers_dsl::id)
        .get_result(&mut conn)
        .await?;

    fs::create_dir_all(Path::new("worlds").join(id.to_string()).join("world")).await?;

    if let Err(e) = process(port, id, &args, ctx, command).await {
        delete(servers_dsl::servers)
            .filter(servers_dsl::name.eq(args.name))
            .execute(&mut pool.get().await?)
            .await?;

        fs::remove_dir_all(Path::new("worlds").join(id.to_string())).await?;

        return Err(e);
    }

    let embed = CreateEmbed::new()
        .description(format!("**Serveur ``{}`` créé !**", args.name))
        .color(EMBED_COLOR);

    command
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new().add_embed(embed),
        )
        .await?;

    log::info!("Created \"{}\" server!", args.name);

    Ok(())
}

async fn process(
    port: i64,
    id: i64,
    args: &Args<'_>,
    ctx: &Context,
    command: &CommandInteraction,
) -> Result<(), ClientError> {
    let mut services = Mapping::new();

    let mut mc = Mapping::new();
    if let Some(ver) = args.ver {
        mc.insert("image", Value::String(docker_image_for(ver).to_owned()));
    } else {
        mc.insert("image", Value::String("itzg/minecraft-server".into()));
    }

    mc.insert("tty", Value::Bool(true));
    mc.insert("stdin_open", Value::Bool(true));
    mc.insert(
        "ports",
        Value::Sequence(vec![Value::String(format!("{port}:25565"))]),
    );

    let mut env = Mapping::new();

    env.insert("EULA", Value::String("TRUE".into()));

    env.insert("OPS", Value::String(parse_key::<String>("ADMIN_PLAYER")?));

    if let Some(version) = args.ver {
        let json = fs::read_to_string("versions.json").await?;
        let versions: Vec<String> = serde_json::from_str(&json)?;

        if versions.iter().any(|e| e == version) {
            env.insert("VERSION", Value::String(version.into()));
        } else {
            return Err(ClientError::Other(format!(
                "{version} n'est pas une version valide."
            )));
        }
    }

    if let Some(difficulty) = args.difficulty_option {
        env.insert("DIFFICULTY", Value::String(difficulty.into()));
    }

    env.insert(
        "MAX_MEMORY",
        Value::String(parse_key::<String>("MAX_MEMORY")?),
    );

    mc.insert(
        "volumes",
        Value::Sequence(vec![
            Value::String("./data:/data".to_string()),
            Value::String("./world:/world".to_string()),
        ]),
    );

    let mut healthcheck = Mapping::new();

    healthcheck.insert("test", Value::String("mc-health".into()));
    healthcheck.insert("start_period", Value::String("1m".into()));
    healthcheck.insert("interval", Value::String("5s".into()));
    healthcheck.insert("retries", Value::String("20".into()));

    mc.insert("healthcheck", Value::Mapping(healthcheck));

    if args.map {
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

        env.insert("WORLD", Value::String(format!("/world/{uuid}")));
    }

    if let Some(mp) = args.modpack {
        env.insert("MODPACK_PLATFORM", Value::String("AUTO_CURSEFORGE".into()));
        env.insert("CF_API_KEY", Value::String(parse_key("CF_API_KEY")?));
        if mp == "cf" {
            let embed = CreateEmbed::new()
                .description("**Veuillez écrire le lien du modpack**".to_string())
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
                env.insert("CF_PAGE_URL", Value::String(url.content.clone()));

                url.delete(&ctx.http).await?;
            } else {
                return Err(ClientError::Other(
                    "Vous n'avez pas écris de lien.".to_string(),
                ));
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

            env.insert("CF_SLUG", Value::String("custom".into()));

            env.insert("CF_MODPACK_ZIP", Value::String(format!("/world/{uuid}")));
        }
    }

    if args.crack {
        env.insert("ONLINE_MODE", Value::Bool(false));
    }

    env.insert("ENABLE_COMMAND_BLOCK", Value::Bool(true));

    env.insert("SPAWN_PROTECTION", Value::Number(0.into()));

    mc.insert("environment", Value::Mapping(env));

    services.insert("mc", Value::Mapping(mc));

    let mut root = Mapping::new();
    root.insert("services", Value::Mapping(services));

    let yml_str = serde_yml::to_string(&root)?;

    fs::write(
        Path::new("worlds")
            .join(id.to_string())
            .join("docker-compose.yml"),
        yml_str,
    )
    .await?;

    Ok(())
}

fn docker_image_for(version: &str) -> &'static str {
    let parts: Vec<u32> = version.split('.').filter_map(|s| s.parse().ok()).collect();

    let minor = *parts.get(1).unwrap_or(&0);

    match minor {
        0..=16 => "itzg/minecraft-server:java8",
        17..=20 => {
            let patch = *parts.get(2).unwrap_or(&0);

            if minor == 20 && patch >= 5 {
                "itzg/minecraft-server:java21"
            } else {
                "itzg/minecraft-server:java17"
            }
        }
        _ => "itzg/minecraft-server:java21",
    }
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
        .add_option(CreateCommandOption::new(
            CommandOptionType::Boolean,
            "crack",
            "Ouvrir au crack ?",
        ))
        .description_localized("en-US", "Open to Cracked?")
        .description_localized("en-GB", "Open to Cracked?")
}
