#![feature(is_some_and)]
#![feature(async_closure)]
#![feature(let_chains)]
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
pub mod commands;


use std::sync::Arc;

#[cfg(not(debug_assertions))]
use {
    std::env
};

use songbird::SerenityInit;

use serenity::{
    async_trait,
    client::{Client, EventHandler, Context},
    framework::StandardFramework,
    model::{
        channel::Message,
        gateway::Ready,
        application::interaction::Interaction
    },
    prelude::GatewayIntents
};

use crate::{
    arcs::{CacheAndHttp, register_cache_and_http},
    interaction::{handle_message},
    json::{load_guilds_to_cache, save_guilds_to_disk},
    commands::{
        setup,
    }
};

use tokio_schedule::Job;
use tracing::log::{error, Level, log};


struct Handler;

#[macro_use]
extern crate lazy_static;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if !msg.is_own(&ctx) {
            handle_message(ctx, msg).await;
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        register_cache_and_http(Arc::new(CacheAndHttp { cache: ctx.cache.clone(), http: ctx.http.clone(), shard: Arc::new(ctx.shard.clone()) })).await;
        log!(Level::Info, "{} is connected! Beginning disk load", ready.user.name);

        match load_guilds_to_cache().await {
            Ok(_) => log!(Level::Info, "Successfully loaded guilds from disk"),
            Err(err) => {
                panic!("Unable to load guilds from disk due to: {} Aborting.", err)
            }
        }

        match commands::register_commands(&ctx.http).await {
            Ok(_) => log!(Level::Info, "Commands successfully registered"),
            Err(err) => error!("Error registering commands {}", err)
        }


        let future = tokio_schedule::every(5).seconds().perform(|| async { save_guilds_to_disk().await});
        tokio::spawn(future);
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            match command.data.name.as_str() {
                setup::SETUP_CMD_NAME => setup::execute(ctx, command).await,
                _ => {}
            };
        }
    }
}

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
            .prefix(prefix)
            .allow_dm(false));


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

    tokio::signal::ctrl_c().await.ok();
    println!("Received Ctrl-C, shutting down.");
}