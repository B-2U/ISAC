use poise::serenity_prelude::{Colour, CreateEmbed};

pub trait EasyEmbed<T> {
    fn default_isac() -> Self;
}
impl EasyEmbed<&mut CreateEmbed> for CreateEmbed {
    /// ISAC default embed colour
    fn default_isac() -> CreateEmbed {
        CreateEmbed::default().colour(Colour::LIGHTER_GREY)
    }
}
