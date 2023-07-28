use poise::serenity_prelude::{Colour, CreateEmbed};

pub trait EasyEmbed<T> {
    fn default_from(input: T) -> T;

    fn default_new() -> Self;
}
impl EasyEmbed<&mut CreateEmbed> for CreateEmbed {
    fn default_from(input: &mut CreateEmbed) -> &mut CreateEmbed {
        input.colour(Colour::LIGHT_GREY);
        input
    }

    fn default_new() -> CreateEmbed {
        let mut embed = CreateEmbed::default();
        embed.colour(Colour::LIGHT_GREY);
        embed
    }
}
