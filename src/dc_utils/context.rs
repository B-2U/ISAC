use std::sync::Arc;

use crate::Context;
use poise::{async_trait, serenity_prelude::Typing};

#[async_trait]
pub trait ContextAddon {
    async fn reply(
        &self,
        input: impl Into<String> + std::marker::Send,
    ) -> Result<poise::ReplyHandle, poise::serenity_prelude::Error>;
    async fn typing(&self) -> MyTyping;
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
    async fn typing(&self) -> MyTyping {
        let typing = Typing::start(
            Arc::clone(&self.serenity_context().http),
            self.channel_id().0,
        )
        .ok();
        MyTyping::new(typing)
    }
}

/// A wrapped serenity typing which impl dropping
pub struct MyTyping {
    pub typing: Option<Typing>,
}

impl MyTyping {
    fn new(typing: Option<Typing>) -> Self {
        Self {
            typing,
        }
    }
}

impl Drop for MyTyping {
    fn drop(&mut self) {
        if let Some(typing) = self.typing.take() {
            let _r = typing.stop();
        }
    }
}
