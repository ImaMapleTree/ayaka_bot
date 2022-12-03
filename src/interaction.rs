pub mod menu;
pub mod menu_defaults;

use std::str::FromStr;




use serenity::client::{Context};



use serenity::model::channel::{Message};
use serenity::model::id::{ChannelId, GuildId};

use tracing::error;
use tracing::log::{Level, log};
use crate::arcs::{get_cache_and_http};
use crate::guild::GUILD_REGISTRY;
use crate::interaction::menu::create_interaction;
use crate::music::state::{MusicState, QueueAction};


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
            let (state, action) = match id {
                "next" | "prev" => (music.change_track(QueueAction::from(id)).await, QueueAction::from(id)),
                "stop" => (music.stop_music().await, QueueAction::HardNext),
                "shuffle" => (music.toggle_shuffle(), QueueAction::StateChange),
                "loop" => (music.toggle_loop(), QueueAction::StateChange),
                "queue_select" => {
                    let index = usize::from_str(interaction.data.values.last().unwrap()).unwrap();
                    music.cut_line(index);
                    (music.change_track(QueueAction::SelectedNext).await, QueueAction::SelectedNext)
                }
                _ => { (music.get_state(None), QueueAction::StateChange) }
            };
            if let Some(interaction) = &mut manager_lock.interaction {
                interaction.update_message(state, action).await;
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
        let cache_and_http = get_cache_and_http().await;
        let messages = match self.channel_id.messages(&cache_and_http.http, |ret| ret.limit(50)).await {
            Ok(messages) => messages,
            Err(err) => { error!("{}", err); return self }
        };

        if messages.iter().filter(|m| !m.is_own(&cache_and_http.cache)).count() > 0 {
            match context {
                None => {}
                Some(ctx) => { self.channel_id.say(&ctx, "❌ **Designated music channel must first be empty** ❌").await.ok(); return self }
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

    pub async fn update_message(&mut self, music_state: MusicState, action: QueueAction) {
        let cache = get_cache_and_http().await;

        if let Some(message) = &mut self.message {
            let mut new_message = match self.channel_id.message(&cache.http, message.id).await {
                Err(_err) => return,
                Ok(message) => message
            };

            let edit_message = match action {
                QueueAction::HardNext | QueueAction::Previous | QueueAction::SelectedNext => {
                    menu::new_menu(music_state)
                }
                QueueAction::SoftNext | QueueAction::StateChange => {
                    menu::modify_menu(&new_message, music_state)
                }
            };

            match new_message.edit(cache, |builder| {
                *builder = edit_message;
                builder
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

pub async fn handle_message(ctx: Context, msg: Message) -> Option<()> {
    let guild_id = msg.guild_id?;
    let acquire_lock = GUILD_REGISTRY.lock().await;
    let guild_manager = acquire_lock.get(&guild_id)?;
    let mut guild_lock = guild_manager.lock().await;
    if msg.channel_id != guild_lock.interaction.as_ref()?.channel_id { return None; }

    msg.delete(&ctx).await.ok();

    let music = &mut guild_lock.music;

    music.try_join(&ctx, &msg, msg.guild(&ctx)).await;

    let search = msg.content;
    if search.ends_with("setup") { return None; }

    if search.starts_with("http") {
        music.queue(search).await;
    } else {
        music.search_and_queue(search).await;
    }

    let (metadata, action) = if !music.is_playing {
        (music.change_track(QueueAction::SoftNext).await, QueueAction::HardNext)
    } else {
        (
            MusicState { metadata: None, queue_names: music.get_items_in_queue(), looping: music.looping, shuffling: music.shuffling },
            QueueAction::StateChange
        )
    };
    guild_lock.interaction.as_mut()?.update_message(metadata, action).await;
    Some(())
}