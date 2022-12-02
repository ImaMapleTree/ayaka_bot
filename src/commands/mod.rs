use std::sync::Arc;
use serenity::builder::CreateInteractionResponse;
use serenity::http::Http;
use serenity::model::prelude::command::Command;
use serenity::prelude::SerenityError;
use tracing::log::{Level, log};

pub mod setup;

pub async fn register_commands(http: &Arc<Http>) -> Result<(), SerenityError> {
    Ok(log!(Level::Info, "Command Registered {:?}", Command::create_global_application_command(http, |b| setup::register(b)).await?))
}

pub fn interaction_msg_response(message: &str, ephemeral: bool) -> CreateInteractionResponse {
    let mut response = CreateInteractionResponse::default();
    response.interaction_response_data(|resp| resp
        .ephemeral(ephemeral)
        .content(message)
    );
    response
}