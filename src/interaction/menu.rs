use std::sync::Arc;
use serenity::builder::{CreateComponents, CreateSelectMenuOption, EditMessage};
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::id::ChannelId;
use crate::interaction::menu_defaults::{default_components, default_embed, MUSIC_EMBED_COLOR, MUSIC_EMBED_IMAGE, MUSIC_EMBED_TITLE};
use crate::music::state::{MusicState, QueueItem};
use crate::troll;

pub fn new_menu(music_state: MusicState) -> EditMessage<'static> {
    let metadata = music_state.metadata.unwrap_or_default();
    let mut default_embed = default_embed();
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

    let mut edit_message = EditMessage::default();
    edit_message
        .set_embed(default_embed)
        .set_components(create_queue_component(&music_state.queue_names));
    edit_message
}

/// Updates info such as looping, shuffling, and queue
pub fn modify_menu(current: &Message, music_state: MusicState) -> EditMessage<'static> {
    let current_embed = current.embeds.get(0).expect("No embeds attached to this message");
    let mut edit_message = EditMessage::default();
    edit_message
        .add_embed(|em| {
            let em = em
                .title(current_embed.title.clone().unwrap_or(MUSIC_EMBED_TITLE.to_string()))
                .image(current_embed.image.clone().map(|img| img.url).unwrap_or(MUSIC_EMBED_IMAGE.to_string()))
                .color(MUSIC_EMBED_COLOR)
                .description(current_embed.description.clone().unwrap_or(troll::random_ayaka_quote().to_string()))
                .footer(|f| f.text(format!("Looping: {} | Shuffling: {}", upcase_bool(music_state.looping), upcase_bool(music_state.shuffling))));
            current_embed.clone().url.map(|url| em.url(url));
            em
        })
        .set_components(create_queue_component(&music_state.queue_names));
    edit_message
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
                .placeholder(format!("View Queue ({} Songs)", options.len()))
                .custom_id("queue_select")
                .options(|opt| opt.set_options(options))
        }));
    default
}

pub async fn create_interaction(channel_id: ChannelId, http: Arc<Http>) -> serenity::Result<Message> {
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