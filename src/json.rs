
use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::Mutex;
use tracing::error;
use crate::guild::{GUILD_REGISTRY, GuildManager};

const GUILD_JSON_FILE: &str = "guild_cache.json";
const BACKUP_GUILD_JSON_FILE: &str = "guild_cache-backup.json";
lazy_static! {
    static ref SAVE_COUNT: AtomicUsize = AtomicUsize::default();
}

#[derive(Serialize, Deserialize)]
pub struct GuildCfgFile {
    pub guilds: Vec<GuildJson>
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct GuildJson {
    pub music_channel: Option<u64>,
    pub channel_setup: bool,
    pub guild_id: u64
}

pub async fn load_guilds_to_cache() -> Result<(), String> {
    let mut input = String::new();

    match File::options().read(true).open(GUILD_JSON_FILE) {
        Ok(mut file) => file.read_to_string(&mut input).ok(),
        Err(err) => return match err.kind() {
            ErrorKind::NotFound => {
                match File::options().truncate(true).write(true).create_new(true).open(GUILD_JSON_FILE) {
                    Ok(_) => Ok(()),
                    Err(err) => Err(err.to_string())
                }
            }
            _ => { Err(err.to_string()) }
        }
    };

    if !input.is_empty() {
        let cfg: GuildCfgFile = match serde_json::from_str(&input) {
            Ok(cfg) => cfg,
            Err(err) => return Err(err.to_string())
        };

        let mut registry = GUILD_REGISTRY.lock().await;
        for guild_json in cfg.guilds {
            let guild_manager = GuildManager::from_json(guild_json).await;
            registry.insert(guild_manager.id, Arc::new(Mutex::new(guild_manager)));
        }
    }
    Ok(())
}

pub async fn save_guilds_to_disk() {
    let mut guilds_file = match File::options().truncate(true).write(true).create(true).open(GUILD_JSON_FILE) {
        Ok(file) => file,
        Err(err) => { error!("Error opening / creating guild cache: {}", err); return}
    };

    let mut guilds = Vec::new();
    for guild in GUILD_REGISTRY.lock().await.values() {
        guilds.push(guild.lock().await.to_json_struct())
    }
    let guild_cfg = GuildCfgFile { guilds };
    let guild_string = match serde_json::to_string(&guild_cfg) {
        Ok(string) => string,
        Err(err) => { error!("Error caching json: {}", err); return }
    };

    guilds_file.write(guild_string.as_bytes()).ok();

    let count = SAVE_COUNT.fetch_add(1, Ordering::SeqCst);
    if count >= 4 {
        match File::options().truncate(true).write(true).create(true).open(BACKUP_GUILD_JSON_FILE) {
            Ok(mut file) => { file.write(guild_string.as_bytes()).ok(); },
            Err(_err) => {}
        };
        SAVE_COUNT.store(0, Ordering::SeqCst);
    }

}