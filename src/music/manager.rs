use std::collections::HashMap;
use std::future::Future;
use std::io::Read;
use std::sync::Arc;
use serenity::async_trait;
use serenity::client::Context;
use serenity::model::channel::Message;
use serenity::model::id::GuildId;
use songbird::{Call, Event, EventContext, EventHandler, Songbird, TrackEvent, ytdl};
use songbird::input::{Input, Restartable, ytdl_search};
use tokio::sync::Mutex;
use tracing_subscriber::fmt::SubscriberBuilder;
use crate::music::discord::join_guild_channel_from_msg;

lazy_static! {
    pub static ref registry: Mutex<HashMap<GuildId, Arc<Mutex<MusicManager>>>> = Mutex::new(HashMap::new());
}

const MAX_QUEUE_HISTORY: usize = 3;

unsafe impl Sync for MusicManager {}

#[derive(Debug)]
pub struct MusicManager {
    queue: Vec<Restartable>,
    handler: Arc<Mutex<Call>>,
    next_track: usize,
    pub is_playing: bool,
}

pub struct TrackEndEvent {
    handler: Arc<Mutex<Call>>,
    id: GuildId
}


#[async_trait]
impl EventHandler for TrackEndEvent {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let event = ctx.to_core_event().map(|c| c.into());
        let manager = MusicManager::get(self.handler.clone(), &self.id).await;
        let mut manager_lock = manager.lock().await;
        manager_lock.play_next(false).await;
        event
    }
}

impl MusicManager {
    pub async fn get_from_ctx(ctx: &Context, msg: &Message, id: &GuildId) -> Option<Arc<Mutex<MusicManager>>> {
        let songbird = match songbird::get(ctx).await {
            Some(bird) => bird,
            None => {
                println!("Could not retrieve songbird instance");
                return None;
            }
        };

        let handler = match songbird.get(*id) {
            Some(handler) => handler,
            None => match join_guild_channel_from_msg(ctx, msg).await.0 {
                Some(handler) => handler,
                None => return None
            }
        };


        Some(MusicManager::get(handler, id).await)
    }

    pub async fn get(handler: Arc<Mutex<Call>>, id: &GuildId) -> Arc<Mutex<MusicManager>> {
        let mut lock = registry.lock().await;
        let cache_manager = lock.get(id);
        match cache_manager {
            Some(manager) => manager.clone(),
            None => {
                let manager = Arc::new(Mutex::new(MusicManager {
                    queue: Vec::new(),
                    handler: handler.clone(),
                    next_track: 0,
                    is_playing: false,
                }));
                handler.clone().lock().await.add_global_event(Event::Track(TrackEvent::End),
                TrackEndEvent {
                    handler,
                    id: id.clone()
                });
                lock.insert(id.clone(), manager.clone());
                manager
            }
        }
    }

    fn trim_queue(&mut self) {
        if self.next_track > MAX_QUEUE_HISTORY {
            self.queue.remove(0);
            self.next_track = self.next_track.saturating_sub(1);
        }
    }

    pub async fn search_and_queue(&mut self, name: String) {
        self.trim_queue();

        self.queue.push(
            match Restartable::ytdl_search(name, true).await {
                Ok(source) => source,
                Err(err) => {
                    eprintln!("{:?}", err);
                    return;
                }
            }
        );
    }

    pub async fn queue(&mut self, url: String) {
        self.trim_queue();

        self.queue.push(
            match Restartable::ytdl(url, true).await {
                Ok(source) => source,
                Err(err) => {
                    eprintln!("{:?}", err);
                    return;
                }
            }
        );
    }

    pub async fn play_next(&mut self, skip: bool) {
        let mut handler_lock = self.handler.lock().await;

        if skip && self.is_playing {
            handler_lock.stop();
            return;
        }

        let track = match self.queue.get(self.next_track) {
            Some(track) => track.clone(),
            None => {
                self.is_playing = false;
                return;
            }
        };

        self.next_track += 1;
        self.is_playing = true;
        handler_lock.play_only_source(track.into());
    }
}