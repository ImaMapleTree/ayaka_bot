use tokio::sync::Mutex;
use std::collections::HashMap;
use std::future::Future;
use std::io::{Read, Seek};
use std::sync::Arc;
use rand::Rng;
use serenity::async_trait;
use serenity::client::Context;
use serenity::model::channel::Message;
use serenity::model::guild::Guild;
use serenity::model::id::GuildId;
use songbird::{Call, Event, EventContext, EventHandler, Songbird, TrackEvent, ytdl};
use songbird::input::{Input, Metadata, Restartable, ytdl_search};
use crate::guild::GUILD_REGISTRY;
use crate::music::discord::{get_user_vc, join_guild_channel_from_msg};
use crate::music::state::{MusicState, QueueAction, QueueItem};

const MAX_QUEUE_HISTORY: usize = 20;

unsafe impl Sync for MusicManager {}

#[derive(Debug)]
pub struct MusicManager {
    queue: Vec<Restartable>,
    handler: Arc<Mutex<Call>>,
    next_track: usize,
    pub is_playing: bool,
    pub guild_id: GuildId,
    pub looping: bool,
    pub shuffling: bool
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
        let mut guild_lock = guild_manager.lock().await;
        let metadata = guild_lock.music.as_mut().expect("No music manager?").change_track(QueueAction::SoftNext).await;
        if let Some(interaction) = &mut guild_lock.interaction {
            interaction.update_message(metadata, true).await;
        }
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
            guild_id,
            looping: false,
            shuffling: false
        });
        handler.clone().lock().await.add_global_event(
            Event::Track(TrackEvent::End),
            TrackEndEvent {
                handler,
                id: guild_id.clone()
            });
        manager
    }

    fn neaten_queue(&mut self) {
        if self.next_track > MAX_QUEUE_HISTORY {
            self.queue.remove(0);
            self.next_track = self.next_track.saturating_sub(1);
        }
    }

    pub async fn search_and_queue(&mut self, name: String) {
        self.neaten_queue();

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
        self.neaten_queue();

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

    pub fn cut_line(&mut self, target: usize) {
        let item = self.queue.remove(target);
        self.queue.insert(self.next_track, item);
    }

    pub fn toggle_loop(&mut self) -> MusicState {
        self.looping = !self.looping;
        if self.looping {
            self.queue = self.queue.split_off(self.next_track);
            self.next_track = 0;
        }
        self.get_state(None)
    }

    pub fn toggle_shuffle(&mut self) -> MusicState {
        self.shuffling = !self.shuffling;
        self.get_state(None)
    }

    pub async fn stop_music(&mut self) -> MusicState {
        self.queue.clear();
        self.change_track(QueueAction::HardNext).await
    }

    pub async fn  change_track(&mut self, action: QueueAction) -> MusicState {
        let mut handler_lock = self.handler.lock().await;

        match action {
            QueueAction::HardNext | QueueAction::SelectedNext => {
                if self.is_playing { handler_lock.stop(); }
            }
            QueueAction::Previous => { self.next_track = self.next_track.saturating_sub(2); }
            _ => {}
        }

        if self.shuffling && action != QueueAction::SelectedNext {
            if !self.queue.is_empty() {
                let mut next_track = rand::thread_rng().gen_range(0..self.queue.len());
                while next_track == self.next_track.saturating_sub(1) && self.next_track.saturating_sub(1) > 0 {
                    next_track = rand::thread_rng().gen_range(0..self.queue.len());
                }
                self.next_track = next_track;
            }
        }

        if self.next_track >= self.queue.len() && self.looping {
            self.next_track = 0
        }

        let track = match self.queue.get(self.next_track) {
            Some(track) => track.clone(),
            None => {
                self.is_playing = false;
                return MusicState {
                    metadata: None,
                    queue_names: vec![],
                    looping: self.looping,
                    shuffling: self.shuffling
                }
            }
        };

        self.next_track += 1;
        self.is_playing = true;
        let input: Input = track.into();
        let metadata = input.metadata.clone();


        handler_lock.play_only_source(input);
        self.get_state(Some(metadata))
    }

    pub fn get_items_in_queue(&self) -> Vec<QueueItem> {
        let skip_amount = if self.shuffling { 0 } else { self.next_track };


        self.queue.iter()
            .enumerate()
            .skip(skip_amount)
            .map(|(i, t)| {
                let input: Input = t.clone().into();
                QueueItem {
                    title: input.metadata.title.unwrap_or(String::from("")),
                    index: i
                }
            }).collect::<Vec<QueueItem>>()
    }

    pub fn get_state(&self, metadata: Option<Box<Metadata>>) -> MusicState {
        MusicState {
            metadata,
            queue_names: self.get_items_in_queue(),
            looping: self.looping,
            shuffling: self.shuffling
        }
    }
}