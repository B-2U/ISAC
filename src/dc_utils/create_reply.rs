// TODO: delete it if no need anymore
use poise::{serenity_prelude::CreateEmbed, CreateReply};

pub trait CreateReplyAddon {
    fn set_embed(self, embed: CreateEmbed) -> Self;

    fn set_embeds(self, embeds: Vec<CreateEmbed>) -> Self;
}

impl CreateReplyAddon for CreateReply {
    /// Existing embeds will be removed
    fn set_embed(mut self, embed: CreateEmbed) -> Self {
        self.embeds = vec![embed];
        self
    }

    fn set_embeds(mut self, embeds: Vec<CreateEmbed>) -> Self {
        self.embeds = embeds;
        self
    }
}
