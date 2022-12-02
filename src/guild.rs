use tokio::sync::{Mutex, MutexGuard};
use std::sync::Arc;
use std::collections::HashMap;
use serenity::async_trait;
use serenity::client::{Context, RawEventHandler};
use serenity::model::event::Event;
use serenity::model::guild::Member;
use serenity::model::id::{ChannelId, GuildId};
use serenity::prelude::EventHandler;
use crate::json::GuildJson;
use crate::member::MemberManager;
use crate::music::manager::MusicManager;
use crate::interaction::InteractionManager;

type GuildRegistry<'a> = MutexGuard<'a, HashMap<GuildId, Arc<Mutex<GuildManager>>>>;

lazy_static! {
    pub static ref GUILD_REGISTRY: Mutex<HashMap<GuildId, Arc<Mutex<GuildManager>>>> = Mutex::new(HashMap::new());
}

#[derive(Debug)]
pub struct GuildManager {
    pub music: Option<MusicManager>,
    pub interaction: Option<InteractionManager>,
    pub member: MemberManager,
    pub id: GuildId
}

impl GuildManager {
    /// Constraint: Should only be called on guild join
    /// Or with specific command
    pub fn new(id: GuildId) -> GuildManager {
        GuildManager {
            music: None,
            interaction: None,
            member: MemberManager::default(),
            id
        }
    }

    pub async fn new_channel(&mut self, ctx: &Context, id: ChannelId) {
        self.interaction = Some(InteractionManager::new(Some(ctx), id).await);
    }

    pub async fn register(self) -> Arc<Mutex<GuildManager>> {
        let id = self.id;
        let arc = Arc::new(Mutex::new(self));
        GUILD_REGISTRY.lock().await.insert(id, arc.clone());
        arc
    }

    pub async fn register_already_locked(self, mut registry: GuildRegistry<'_>) -> Arc<Mutex<GuildManager>> {
        let id = self.id;
        let arc = Arc::new(Mutex::new(self));
        registry.insert(id, arc.clone());
        arc
    }

    pub async fn init_async(&mut self) {
        self.interaction = match &self.interaction {
            None => None,
            Some(manager) => Some(InteractionManager::new(None, manager.channel_id).await)
        };
    }

    pub async fn from_json(json: GuildJson) -> Self {
        let interaction = if !json.channel_setup || json.music_channel.is_none() { None }
        else {
            Some(InteractionManager::new(None, ChannelId(json.music_channel.unwrap())).await)
        };
        GuildManager {
            music: None,
            interaction,
            member: MemberManager::default(),
            id: GuildId(json.guild_id)
        }
    }

    pub fn to_json_struct(&self) -> GuildJson {
        GuildJson {
            music_channel: self.interaction.as_ref().map(|r| r.channel_id.0),
            channel_setup: self.interaction.as_ref().is_some_and(|i| i.message.is_some()),
            guild_id: self.id.0
        }
    }
}

impl From<GuildJson> for GuildManager {
    fn from(value: GuildJson) -> Self {
        GuildManager {
            music: None,
            interaction: value.music_channel.map(|v| InteractionManager::new_no_async( ChannelId(v))),
            member: MemberManager::default(),
            id: GuildId(value.guild_id)
        }
    }
}

