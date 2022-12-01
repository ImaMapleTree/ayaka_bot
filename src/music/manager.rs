use tokio::sync::Mutex;
use std::collections::HashMap;
use std::future::Future;
use std::io::Read;
use std::sync::Arc;
use serenity::async_trait;
use serenity::client::Context;
use serenity::model::channel::Message;
use serenity::model::guild::Guild;
use serenity::model::id::GuildId;
use songbird::{Call, Event, EventContext, EventHandler, Songbird, TrackEvent, ytdl};
use songbird::input::{Input, Metadata, Restartable, ytdl_search};
use crate::guild::GUILD_REGISTRY;
use crate::music::discord::{get_user_vc, join_guild_channel_from_msg};

const MAX_QUEUE_HISTORY: usize = 3;

unsafe impl Sync for MusicManager {}

#[derive(Debug)]
pub struct MusicManager {
    queue: Vec<Restartable>,
    handler: Arc<Mutex<Call>>,
    next_track: usize,
    pub is_playing: bool,
    pub guild_id: GuildId,
}

pub struct TrackEndEvent {
    handler: Arc<Mutex<Call>>,
    id: GuildId
}


#[async_trait]
impl EventHandler for TrackEndEvent {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let event = ctx.to_core_event().map(|c| c.into());
        let registry_lock = GUILD_REGISTRY.lock().await;
        let guild_manager = registry_lock.get(&self.id)?.clone();
        guild_manager.lock().await.music.as_mut().expect("No music manager?").play_next(false).await;
        event
    }
}

impl MusicManager {
    pub async fn new(ctx: &Context, msg: &Message, guild_id: GuildId) -> Option<MusicManager> {
        let songbird = match songbird::get(ctx).await {
            Some(bird) => bird,
            None => {
                println!("Could not retrieve songbird instance");
                return None;
            }
        };

        let mut handler = match songbird.get(guild_id) {
            Some(handler) => handler,
            None => join_guild_channel_from_msg(ctx, msg).await.0?
        };

        let manager = Some(MusicManager {
            queue: Vec::new(),
            handler: handler.clone(),
            next_track: 0,
            is_playing: false,
            guild_id
        });
        handler.clone().lock().await.add_global_event(
            Event::Track(TrackEvent::End),
            TrackEndEvent {
                handler,
                id: guild_id.clone()
            });
        manager
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

    pub async fn play_next(&mut self, skip: bool) -> Option<Box<Metadata>> {
        println!("Play next, {}, {}", skip, self.is_playing);
        let mut handler_lock = self.handler.lock().await;

        if skip && self.is_playing {
            handler_lock.stop();
        }

        let track = match self.queue.get(self.next_track) {
            Some(track) => track.clone(),
            None => {
                self.is_playing = false;
                return None;
            }
        };

        self.next_track += 1;
        self.is_playing = true;
        let input: Input = track.into();
        let metadata = input.metadata.clone();


        println!("Playing next track: {:?}", input.metadata.title);
        handler_lock.play_only_source(input); Some(metadata)
    }

}