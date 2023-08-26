use poise::serenity_prelude::{Colour, CreateEmbed};

pub trait EasyEmbed<T> {
    fn default_from(input: T) -> T;

    fn isac() -> Self;
}
impl EasyEmbed<&mut CreateEmbed> for CreateEmbed {
    fn default_from(input: &mut CreateEmbed) -> &mut CreateEmbed {
        input.colour(Colour::LIGHT_GREY);
        input
    }
    /// ISAC default embed colour
    fn isac() -> CreateEmbed {
        let mut embed = CreateEmbed::default();
        embed.colour(Colour::LIGHT_GREY);
        embed
    }
}
