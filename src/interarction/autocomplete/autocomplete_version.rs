use crate::{
    client::error::ClientError, interarction::autocomplete::create_autocomplete_from_vec,
    util::data_types::VersionsList,
};
use serenity::all::{CommandInteraction, Context, ResolvedValue};

pub async fn autocomplete_version(
    ctx: Context,
    command: CommandInteraction,
) -> Result<(), ClientError> {
    let options = command.data.options();
    let opt = &options
        .iter()
        .find(|opt| opt.name == "version")
        .unwrap()
        .value;
    match opt {
        ResolvedValue::Autocomplete {
            kind: _,
            value: str,
        } => {
            let versions: Vec<String> =
                ctx.data.read().await.get::<VersionsList>().unwrap().clone();

            let autocomplete = create_autocomplete_from_vec(versions, str);

            command
                .create_response(
                    &ctx.http,
                    serenity::all::CreateInteractionResponse::Autocomplete(autocomplete),
                )
                .await?;
            Ok(())
        }
        _ => Err(ClientError::Other("invalid value".to_string())),
    }
}
