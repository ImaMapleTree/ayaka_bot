use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize)]
pub struct GuildCfgFile {
    pub guilds: Vec<GuildJson>
}

#[derive(Serialize, Deserialize)]
pub struct GuildJson {
    pub music_channel: Option<u64>,
    pub guild_id: u64
}