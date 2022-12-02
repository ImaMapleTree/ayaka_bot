use serenity::builder::{CreateApplicationCommand, CreateInteractionResponse};
use serenity::client::Context;
use serenity::model::application::interaction::application_command::ApplicationCommandInteraction;
use crate::commands::interaction_msg_response;
use crate::guild::{GUILD_REGISTRY, GuildManager};

pub const SETUP_CMD_NAME: &str = "setup";
pub const SETUP_CMD_DESC: &str = "Use in an empty channel to designate it as this guild's music channel";

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name(SETUP_CMD_NAME).description(SETUP_CMD_DESC)
}

pub async fn execute(ctx: Context, interaction: ApplicationCommandInteraction) {
    let acquire_lock = GUILD_REGISTRY.lock().await;

    let guild_id = match interaction.guild_id {
        None => return,
        Some(guild_id) => guild_id
    };

    let manager = acquire_lock.get(&guild_id);

    let manager = match manager {
        Some(manager) => manager.clone(),
        None => {
            GuildManager::new(guild_id).register_already_locked(acquire_lock).await
        }
    };
    manager.lock().await.new_channel(&ctx, interaction.channel_id).await;
    interaction.create_interaction_response(&ctx.http, |i| {
        *i = interaction_msg_response("Successfully setup channel", true); i
    }).await.ok();
}