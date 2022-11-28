use std::future::Future;
use std::sync::Arc;
use serenity::client::Context;
use serenity::model::channel::{Channel, GuildChannel, Message};
use serenity::model::guild::Guild;
use serenity::model::id::ChannelId;
use serenity::model::user::User;
use songbird::{Call, Songbird};
use songbird::error::JoinResult;
use songbird::model::id::GuildId;
use tokio::sync::Mutex;

pub async fn join_channel(songbird: Arc<Songbird>, channel: GuildChannel) -> Arc<Mutex<Call>> {
    let result = songbird.join(channel.guild_id, channel.id).await;
    println!("Discord Channel Result: {:?}", result);
    result.0
}

pub async fn join_guild_channel_from_msg(ctx: &Context, message: &Message) -> (Option<Arc<Mutex<Call>>>, Option<Arc<Songbird>>) {
    let guild = match message.guild(&ctx) {
        Some(guild) => guild,
        None => {
            message.reply(ctx, "Cannot join non-guild channel");
            return (None, None);
        }
    };
    let guild_id = guild.id;
    let channel_id = match get_user_vc(guild, message.author.clone()) {
        None => return (None, None),
        Some(channel_id) => channel_id
    };

    let songbird = songbird::get(ctx).await.expect("Unable to get Songbird");
    (Some(songbird.clone().join(guild_id, channel_id).await.0), Some(songbird))
}

pub fn get_user_vc(guild: Guild, user: User) -> Option<ChannelId> {
    guild.voice_states.get(&user.id).and_then(|state| state.channel_id)
}