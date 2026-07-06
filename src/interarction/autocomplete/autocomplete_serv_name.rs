use crate::client::error::ClientError;
use crate::interarction::autocomplete::create_autocomplete_from_vec;
use crate::util::data_types::ServNames;
use serenity::all::{CommandInteraction, Context, ResolvedValue};

pub async fn autocomplete_serv_name(
    ctx: Context,
    command: CommandInteraction,
) -> Result<(), ClientError> {
    let options = command.data.options();
    let opt = &options.iter().find(|opt| opt.name == "name").unwrap().value;

    match opt {
        ResolvedValue::Autocomplete {
            kind: _,
            value: str,
        } => {
            let names: Vec<String> = ctx.data.read().await.get::<ServNames>().unwrap().clone();

            let autocomplete = create_autocomplete_from_vec(names, str);

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
