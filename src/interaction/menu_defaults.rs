use serenity::builder::{CreateComponents, CreateEmbed};
use serenity::model::application::component::ButtonStyle;
use serenity::utils::Color;
use crate::troll;

pub const MUSIC_EMBED_TITLE: &str = "No song currently playing";
pub const MUSIC_EMBED_COLOR: Color = Color::from_rgb(120, 107, 199);
pub const MUSIC_EMBED_IMAGE: &str = "https://cdn.discordapp.com/attachments/893017931087245325/1047929174406471711/ayaka.PNG";
pub const MUSIC_EMBED_FOOTER_TEXT: &str = "Looping: False | Shuffling: False";

pub fn default_embed() -> CreateEmbed {
    let mut embed = CreateEmbed::default();
    embed.title(MUSIC_EMBED_TITLE)
        .description(troll::random_ayaka_quote())
        .color(MUSIC_EMBED_COLOR)
        .image(MUSIC_EMBED_IMAGE)
        .footer(|footer| footer.text(MUSIC_EMBED_FOOTER_TEXT));
    embed

}

pub fn default_components() -> CreateComponents {
    let mut components = CreateComponents::default();
    components.create_action_row(|row| row
        .create_button(|button| button
            .style(ButtonStyle::Primary)
            .custom_id("prev")
            .emoji('⏮')
        )
        .create_button(|button| button
            .style(ButtonStyle::Primary)
            .custom_id("next")
            .emoji('⏭')
        )
        .create_button(|button| button
            .style(ButtonStyle::Primary)
            .custom_id("stop")
            .emoji('⏹')
        )
        .create_button(|button| button
            .style(ButtonStyle::Secondary)
            .custom_id("loop")
            .emoji('🔁')
        )
        .create_button(|button| button
            .style(ButtonStyle::Secondary)
            .custom_id("shuffle")
            .emoji('🔀')
        )
    ).create_action_row(|row| row
        .create_button(|button| button
            .style(ButtonStyle::Success)
            .custom_id("APY")
            .label("Add to Playlist")
        )
    );
    components
}