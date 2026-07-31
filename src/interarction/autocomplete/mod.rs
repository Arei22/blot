mod autocomplete_serv_name;
mod autocomplete_version;

use crate::client::error::ClientError;
use autocomplete_serv_name::autocomplete_serv_name;
use autocomplete_version::autocomplete_version;
use serenity::all::{CommandInteraction, Context, CreateAutocompleteResponse};

const MAX_SUGESTION: u16 = 25;

pub async fn autocomplete(ctx: Context, command: CommandInteraction) -> Result<(), ClientError> {
    match command.data.name.as_str() {
        "create" => autocomplete_version(ctx, command).await,
        "list" => Ok(()),
        "delete" | "start" | "stop" | "info" => autocomplete_serv_name(ctx, command).await,
        _ => Err(ClientError::OtherStatic(
            "Slash command defined at Discord but not in the bot.",
        )),
    }
}

pub fn create_autocomplete_from_vec(v: Vec<String>, str: &str) -> CreateAutocompleteResponse {
    let values: Vec<String> = v
        .into_iter()
        .filter(|ver| ver.contains(str))
        .take(MAX_SUGESTION.into())
        .collect();

    let mut autocomplete = CreateAutocompleteResponse::new();

    for value in values {
        autocomplete = autocomplete.add_string_choice(&value, &value);
    }

    autocomplete
}
