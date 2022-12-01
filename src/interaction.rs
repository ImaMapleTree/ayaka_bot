use std::sync::Arc;
use regex::Regex;
use serenity::async_trait;
use serenity::builder::CreateEmbed;
use serenity::cache::Cache;
use serenity::client::{Context, EventHandler};
use serenity::http::Http;
use serenity::model::application::component::ButtonStyle;
use serenity::model::application::interaction::message_component::MessageComponentInteraction;
use serenity::model::channel::{Channel, EmbedImage, Message, Reaction, ReactionType};
use serenity::model::id::{ChannelId, GuildId};
use serenity::utils::Color;
use songbird::input::Metadata;
use tracing::error;
use tracing::log::{Level, log};
use crate::arcs::{get_cache_and_http};
use crate::guild::GUILD_REGISTRY;
use crate::music::manager::MusicManager;
use crate::troll;

pub struct InteractionHandler;

impl InteractionHandler {
    pub async fn handle(guild_id: Option<GuildId>, message: Message) {
        log!(Level::Info, "Setup Interaction Handle in {}", guild_id.unwrap_or(GuildId(0)));
        let chs = get_cache_and_http().await.clone();
        let manager = GUILD_REGISTRY.lock().await.get(&guild_id.expect("Not a guild message")).expect("Guild not registered").clone();

        loop {
            let interaction = match message.await_component_interaction(&chs.shard).await {
                None => continue,
                Some(interaction) => interaction
            };
            match interaction.data.custom_id.as_str() {
                "next" => {
                    let mut manager_lock = manager.lock().await;
                    let metadata = if let Some(music) = &mut manager_lock.music {
                        music.play_next(true).await
                    } else { None };
                    if let Some(interaction) = &mut manager_lock.interaction {
                        interaction.update_message(metadata).await;
                    }
                },
                "prev" => {

                },
                "pause" => {

                },
                "stop" => {

                },
                "shuffle" => {

                },
                "loop" => {

                }
                _ => {}
            }

            interaction.defer(&chs.http).await.ok();
        }
    }
}

#[derive(Debug)]
pub struct InteractionManager {
    pub channel_id: ChannelId,
    pub message: Option<Message>
}

impl InteractionManager {
    pub fn new_no_async(channel_id: ChannelId) -> InteractionManager {
        InteractionManager {
            channel_id,
            message: None
        }
    }

    // Only called when creating a new guild either through json or register
    pub async fn new(context: Option<&Context>, channel_id: ChannelId) -> InteractionManager {
        InteractionManager::new_no_async(channel_id).attach_message(context).await
    }

    pub async fn attach_message(mut self, context: Option<&Context>) -> Self {
        println!("Attaching message");
        let cache_and_http = get_cache_and_http().await;
        let messages = match self.channel_id.messages(&cache_and_http.http, |ret| ret.limit(50)).await {
            Ok(messages) => messages,
            Err(err) => { error!("{}", err); return self }
        };

        if messages.iter().filter(|m| !m.is_own(&cache_and_http.cache)).count() > 0 {
            match context {
                None => {}
                Some(ctx) => { self.channel_id.say(&ctx, "❌ **Designated music channel must first be empty** ❌").await; return self }
            }
        }
        for message in messages {
            message.delete(&cache_and_http).await.ok();
        };

        let message = match create_interaction(self.channel_id, cache_and_http.http.clone()).await {
            Ok(message) => message,
            Err(err) => { error!("{}", err); return self }
        };

        self.message = Some(message.clone());

        println!("Got message");
        let guild_id = match match self.channel_id.to_channel_cached(&cache_and_http.cache) {
            None => self.channel_id.to_channel(&cache_and_http.http).await.ok(),
            Some(channel) => Some(channel)
        } {
            None => None,
            Some(channel) => channel.guild().map(|channel| channel.guild_id)
        };
        tokio::spawn(async move { InteractionHandler::handle(guild_id, message).await });
        self
    }

    pub async fn update_message(&mut self, metadata: Option<Box<Metadata>>) {
        let metadata = metadata.unwrap_or_default();
        let cache = get_cache_and_http().await;



        let mut new_message = None;
        if let Some(message) = &mut self.message {
            new_message = match self.channel_id.message(&cache.http, message.id).await {
                Err(err) => return,
                Ok(message) => Some(message)
            };
            let mut default_embed = default_embed();
            let pattern = Regex::new("\\?.*").unwrap();
            metadata.thumbnail.map(|str| default_embed.image(str));
            metadata.title.map(|str| "**".to_owned() + &str + "**").map(|str| default_embed.title(str));
            println!("Thumbnail: {:?}", default_embed);
            println!("New message: {:?}", new_message);
            println!("Result: {:?}", new_message.unwrap().edit(cache, |builder| builder.set_embed(default_embed)).await);
        }
    }
}

pub async fn handle_message(ctx: Context, msg: Message) -> Option<()> {
    let guild_id = msg.guild_id?;
    let acquire_lock = GUILD_REGISTRY.lock().await;
    let guild_manager = acquire_lock.get(&guild_id)?;
    let mut guild_lock = guild_manager.lock().await;
    if msg.channel_id != guild_lock.interaction.as_ref()?.channel_id { return None; }

    msg.delete(&ctx).await.ok();

    if guild_lock.music.is_none() {
        guild_lock.music = MusicManager::new(&ctx, &msg, guild_id).await
    }

    let mut music = guild_lock.music.as_mut()?;
    let search = msg.content;
    if search.ends_with("setup") { return None; }

    if search.starts_with("http") {
        music.queue(search).await;
    } else {
        music.search_and_queue(search).await;
    }

    if !music.is_playing {
        let metadata= music.play_next(false).await;
        guild_lock.interaction.as_mut()?.update_message(metadata).await;
    }

    Some(())
}

pub fn default_embed() -> CreateEmbed {
    let mut embed = CreateEmbed::default();
    embed.title("No song currently playing")
        .description(troll::random_ayaka_quote())
        .color(Color::from_rgb(120, 107, 199))
        .image("https://cdn.discordapp.com/attachments/893017931087245325/1047929174406471711/ayaka.PNG")
        .footer(|footer| footer.text("Looping: False | Shuffling: False"));
    embed

}

async fn create_interaction(channel_id: ChannelId, http: Arc<Http>) -> serenity::Result<Message> {
    channel_id.send_message(&http, |builder| {
        builder.content("**__Queue List__**\nJoin a voice channel and queue songs by name or url by posting in this channel.")
            .embed(|e| { *e = default_embed(); e})
            .components(|interaction| {
            interaction.create_action_row(|row| row
                .create_button(|button| button
                    .style(ButtonStyle::Primary)
                    .custom_id("pause")
                    .emoji('⏯')
                )
                .create_button(|button| button
                    .style(ButtonStyle::Primary)
                    .custom_id("stop")
                    .emoji('⏹')
                )
                .create_button(|button| button
                    .style(ButtonStyle::Secondary)
                    .custom_id("prev")
                    .emoji('⏮')
                )
                .create_button(|button| button
                    .style(ButtonStyle::Primary)
                    .custom_id("next")
                    .emoji('⏭')
                )
            ).create_action_row(|row| row
                .create_button(|button| button
                    .style(ButtonStyle::Secondary)
                    .custom_id("loop")
                    .emoji('🔁')
                )
                .create_button(|button| button
                    .style(ButtonStyle::Secondary)
                    .custom_id("shuffle")
                    .emoji('🔀')
                )
                .create_button(|button| button
                    .style(ButtonStyle::Success)
                    .custom_id("APY")
                    .label("Add to Playlist")
                )
            )
        })
    }).await
}
