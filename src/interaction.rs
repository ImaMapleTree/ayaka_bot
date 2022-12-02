use std::str::FromStr;
use std::sync::Arc;
use serenity::async_trait;
use serenity::builder::{CreateComponents, CreateEmbed, CreateSelectMenuOption, CreateSelectMenuOptions};
use serenity::cache::Cache;
use serenity::client::{Context, EventHandler};
use serenity::http::Http;
use serenity::model::application::component::ButtonStyle;
use serenity::model::application::interaction::message_component::MessageComponentInteraction;
use serenity::model::channel::{Channel, EmbedImage, Message, Reaction, ReactionType};
use serenity::model::guild::Options;
use serenity::model::id::{ChannelId, GuildId};
use serenity::utils::Color;
use songbird::input::Metadata;
use tracing::error;
use tracing::log::{Level, log};
use crate::arcs::{get_cache_and_http};
use crate::guild::GUILD_REGISTRY;
use crate::music::manager::MusicManager;
use crate::music::state::{MusicState, QueueAction, QueueItem};
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
            println!("Interaction data: {:?}", interaction.data);
            let mut manager_lock = manager.lock().await;
            let id = interaction.data.custom_id.as_str();
            let music = &mut manager_lock.music;
            let state = match id {
                "next" | "prev" => match music {
                    Some(music) => Some(music.change_track(QueueAction::from(id)).await),
                    None => None
                },
                "stop" => match music {
                    Some(music) => Some(music.stop_music().await),
                    None => None
                },
                "shuffle" => match music {
                    None => None,
                    Some(music) => Some(music.toggle_shuffle())
                }
                "loop" => match music {
                    None => None,
                    Some(music) => Some(music.toggle_loop())
                }
                "queue_select" => match music {
                    None => None,
                    Some(music) => {
                        let index = usize::from_str(interaction.data.values.last().unwrap()).unwrap();
                        music.cut_line(index);
                        Some(music.change_track(QueueAction::SelectedNext).await)
                    }
                }
                _ => None
            };
            if let Some(interaction) = &mut manager_lock.interaction {
                if let Some(state) = state {
                    interaction.update_message(state, true).await;
                }
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

    pub async fn update_message(&mut self, music_state: MusicState, next_track: bool) {
        let metadata = music_state.metadata.unwrap_or_default();
        let cache = get_cache_and_http().await;


        let mut new_message = None;
        if let Some(message) = &mut self.message {
            new_message = match self.channel_id.message(&cache.http, message.id).await {
                Err(err) => return,
                Ok(message) => Some(message)
            };
            let mut default_embed = default_embed();
            if next_track {
                metadata.thumbnail.map(|str| default_embed.image(str));
                metadata.title.map(|str| "**".to_owned() + &str + "**").map(|str| default_embed.title(str));
                metadata.source_url.map(|url| default_embed.url(url));
                let duration = match metadata.duration {
                    Some(duration) => {
                        let seconds = duration.as_secs() % 60;
                        let minutes = (duration.as_secs() / 60) % 60;
                        let hours = (duration.as_secs() / 60) / 60;
                        let hrs = if hours == 0 { String::from("") } else if hours < 10 { format!("0{}:", hours) } else { format!("{}:", hours) };

                        let mins = if minutes < 10 { format!("0{}:", minutes) } else { format!("{}:", minutes) };

                        let secs = if seconds < 10 { format!("0{}", seconds) } else { format!("{}", seconds) };

                        format!("**Duration:** {}{}{}", hrs, mins, secs)
                    }
                    None => String::from("**Duration:** N/A")
                };
                default_embed.footer(|f| f.text(format!("Looping: {} | Shuffling: {}", upcase_bool(music_state.looping), upcase_bool(music_state.shuffling))));
                let uploader = match metadata.artist {
                    None => String::from("**Uploader:** N/A"),
                    Some(ref uploader) => format!("**Uploader:** {}", uploader)
                };

                if metadata.duration.is_some() || metadata.artist.is_some() {
                    default_embed.description(format!("{} | {}", duration, uploader));
                }
            }

            match new_message.unwrap().edit(cache, |builder| {
                if next_track { builder.set_embed(default_embed); }
                builder.set_components(create_queue_component(&music_state.queue_names))
            }
            ).await {
                Ok(_) => {}
                Err(err) => {
                    error!("Unable to edit music embed: {}", err);
                }
            }
        }
    }
}

pub fn create_queue_component(queue_items: &Vec<QueueItem>) -> CreateComponents {
    let mut default = default_components();
    if queue_items.is_empty() { return default; }

    let mut options = Vec::new();
    for (i, item) in queue_items.iter().enumerate() {
        let mut title = item.title.clone();
        title.truncate(85);
        options.push(CreateSelectMenuOption::new(format!("{}) {}", i+1, title), item.index));
        if i == 24 { break; }
    }


    default.create_action_row(|queue_row|
        queue_row.create_select_menu(|queue_menu| {
            queue_menu
                .placeholder("View Queue")
                .custom_id("queue_select")
                .options(|opt| opt.set_options(options))
        }));
    default
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

    let (metadata, next_track) = if !music.is_playing {
        (music.change_track(QueueAction::SoftNext).await, true)
    } else {
        (
            MusicState { metadata: None, queue_names: music.get_items_in_queue(), looping: music.looping, shuffling: music.shuffling },
            false
        )
    };
    guild_lock.interaction.as_mut()?.update_message(metadata, next_track).await;
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

pub fn default_components() -> CreateComponents {
    let mut components = CreateComponents::default();
    components.create_action_row(|row| row
            .create_button(|button| button
                .style(ButtonStyle::Primary)
                .custom_id("prev")
                .emoji('⏮')
            )
            .create_button(|button| button
                .style(ButtonStyle::Primary)
                .custom_id("next")
                .emoji('⏭')
            )
            .create_button(|button| button
                .style(ButtonStyle::Primary)
                .custom_id("stop")
                .emoji('⏹')
            )
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
        ).create_action_row(|row| row
        .create_button(|button| button
            .style(ButtonStyle::Success)
            .custom_id("APY")
            .label("Add to Playlist")
        )
    );
    components
}

async fn create_interaction(channel_id: ChannelId, http: Arc<Http>) -> serenity::Result<Message> {
    channel_id.send_message(&http, |builder| builder
        //.content("**__Queue List__**\nJoin a voice channel and queue songs by name or url by posting in this channel.")
        .set_embed(default_embed())
        .set_components(default_components())//.create_action_row(|row| row.create_select_menu(|menu| menu.placeholder("Queue").custom_id("Queue").options(|o| {*o = test_option(); o})))
        ).await
}

fn upcase_bool(b: bool) -> String {
    match b {
        true => String::from("True"),
        false => String::from("False")
    }
}