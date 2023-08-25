use poise::{
    serenity_prelude::{CreateComponents, CreateEmbed},
    CreateReply,
};

pub trait CreateReplyAddon {
    fn set_components(&mut self, components: CreateComponents) -> &mut Self;

    fn set_embed(&mut self, embed: CreateEmbed) -> &mut Self;

    fn set_embeds(&mut self, embeds: Vec<CreateEmbed>) -> &mut Self;
}

impl CreateReplyAddon for CreateReply<'_> {
    fn set_components(&mut self, components: CreateComponents) -> &mut Self {
        self.components = Some(components);
        self
    }

    fn set_embed(&mut self, embed: CreateEmbed) -> &mut Self {
        self.embeds = vec![embed];
        self
    }

    fn set_embeds(&mut self, embeds: Vec<CreateEmbed>) -> &mut Self {
        self.embeds = embeds;
        self
    }
}
