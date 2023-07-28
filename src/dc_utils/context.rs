use crate::Context;
use poise::async_trait;

#[async_trait]
pub trait ContextAddon {
    async fn reply(
        &self,
        input: impl Into<String> + std::marker::Send,
    ) -> Result<poise::ReplyHandle, poise::serenity_prelude::Error>;
}

/// a trait for `reply`
#[async_trait]
impl ContextAddon for Context<'_> {
    async fn reply(
        &self,
        content: impl Into<String> + std::marker::Send,
    ) -> Result<poise::ReplyHandle<'_>, poise::serenity_prelude::Error> {
        self.send(|b| b.content(content).reply(true)).await
    }
}
