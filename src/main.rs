#![feature(is_some_and)]
#![feature(async_closure)]
//! Requires the "client", "standard_framework", and "voice" features be enabled in your
//! Cargo.toml, like so:
//!
//! ```toml
//! [dependencies.serenity]
//! git = "https://github.com/serenity-rs/serenity.git"
//! features = ["client", "standard_framework", "voice"]
//! ```
pub mod music;
pub mod interaction;
pub mod guild;
pub mod member;
pub mod json;
pub mod arcs;
pub mod troll;
pub mod details;

use std::env;
use std::env::VarError;
use std::sync::Arc;

// This trait adds the `register_songbird` and `register_songbird_with` methods
// to the client builder below, making it easy to install this voice client.
// The voice client can be retrieved in any command using `songbird::get(ctx).await`.
use songbird::SerenityInit;

// Import the `Context` to handle commands.
use serenity::client::Context;

use serenity::{
    async_trait,
    client::{Client, EventHandler},
    framework::{
        StandardFramework,
        standard::{
            Args, CommandResult,
            macros::{command, group},
        },
    },
    model::{channel::Message, gateway::Ready},
    prelude::GatewayIntents,
    Result as SerenityResult,
};
use serenity::model::channel::AttachmentType::Path;
use serenity::model::channel::Reaction;
use serenity::model::guild::Member;
use serenity::model::id::GuildId;
use songbird::input::{Metadata, Restartable};
use tokio::sync::Mutex;
use tracing::log::{Level, log};
use crate::arcs::{CacheAndHttp, register_cache_and_http};
use crate::guild::{GUILD_REGISTRY, GuildManager};
use crate::interaction::{handle_message, InteractionManager};
use crate::music::manager::MusicManager;


struct Handler;

#[macro_use]
extern crate lazy_static;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        println!("Message Received");
        if !msg.is_own(&ctx) {
            handle_message(ctx, msg).await;
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        register_cache_and_http(Arc::new(CacheAndHttp { cache: ctx.cache.clone(), http: ctx.http.clone(), shard: Arc::new(ctx.shard.clone()) })).await;
        log!(Level::Info, "{} is connected!", ready.user.name);
    }
}

#[group]
#[commands(play, stop, skip, setup)]
struct General;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Configure the client with your Discord bot token in the environment.

    let mut token = String::new();
    let mut prefix = "~";
    cfg_if::cfg_if! {
        if #[cfg(debug_assertions)] {
            token = "MjAwOTkyNTc3MTc5MjIyMDE2.GNJAcw.DoztZ5WLN9QpOVRhDZIRy6Q5ogYFQCCsEKVSFA".to_string();
            prefix = "e!";
        } else {
            token = match env::var("bot_token") {
                Ok(token) => token,
                Err(_) => {
                    panic!("Unable to recover main api key");
                }
            };
        }
    };

    let framework = StandardFramework::new()
        .configure(|c| c
            .prefix(prefix))
        .group(&GENERAL_GROUP);

    let intents = GatewayIntents::all();

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Err creating client");


    tokio::spawn(async move {
        let _ = client.start().await.map_err(|why| println!("Client ended: {:?}", why));
    });

    tokio::signal::ctrl_c().await;
    println!("Received Ctrl-C, shutting down.");
}

#[command]
async fn setup(ctx: &Context, msg: &Message) -> CommandResult {
    let acquire_lock = GUILD_REGISTRY.lock().await;

    let guild_id = match msg.guild_id {
        None => return Ok(()),
        Some(guild_id) => guild_id
    };
    msg.delete(&ctx).await.ok();

    let manager = acquire_lock.get(&guild_id).clone();

    let manager = match manager {
        Some(manager) => manager.clone(),
        None => {
            GuildManager::new(guild_id).register_already_locked(acquire_lock).await
        }
    };
    println!("Manager: {:?}", manager);
    let mut manager_lock = manager.lock().await;
    manager_lock.new_channel(ctx, msg.channel_id).await;
    Ok(())
}

#[command]
async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    println!("Skip called");
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let registry_lock = GUILD_REGISTRY.lock().await;
    let guild_manager = match registry_lock.get(&guild_id) {
        None => return Ok(()),
        Some(guild_manager) => guild_manager,
    };
    let mut guild_lock = guild_manager.lock().await;

    let metadata = match &mut guild_lock.music {
        None => None,
        Some(music) => Some(music.play_next(true).await)
    };

    match metadata {
        None => {}
        Some(metadata) => {
            match &mut guild_lock.interaction {
                None => {}
                Some(interaction) => interaction.update_message(metadata).await
            }
        }
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let search = match args.single::<String>() {
        Ok(url) => url,
        Err(_) => {
            check_msg(msg.channel_id.say(&ctx.http, "Must provide a URL to a video or audio").await);

            return Ok(());
        },
    };

    let registry_lock = GUILD_REGISTRY.lock().await;
    let guild_manager = match registry_lock.get(&guild_id) {
        None => return Ok(()),
        Some(guild_manager) => guild_manager,
    };
    let mut guild_lock = guild_manager.lock().await;
    if guild_lock.music.is_none() {
        guild_lock.music = MusicManager::new(ctx, msg, guild_id).await
    }

    let music_manager = guild_lock.music.as_mut().unwrap();
    if search.starts_with("http") {
        music_manager.queue(search).await;
    } else {
        music_manager.search_and_queue(search).await;
    }

    if !music_manager.is_playing {
        music_manager.play_next(false).await;
    }

    Ok(())
}

#[command]
async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at initialisation.").clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        handler.stop();
    } else {
        check_msg(msg.channel_id.say(&ctx.http, "Not in a voice channel to play in").await);
    }

    Ok(())
}

/// Checks that a message successfully sent; if not, then logs why to stdout.
fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}