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
        match self {
            Context::Prefix(prefix_ctx) => {
                let typing = Typing::start(
                    Arc::clone(&prefix_ctx.serenity_context.http),
                    self.channel_id().0,
                )
                .ok();
                MyTyping::Typing(typing)
            }
            Context::Application(app_ctx) => {
                let _r = app_ctx.defer().await;
                MyTyping::Thinking
            }
        }
    }
}

/// A wrapped serenity typing which impl dropping
pub enum MyTyping {
    Typing(Option<Typing>),
    Thinking,
}

impl MyTyping {
    pub fn stop(self) {
        if let MyTyping::Typing(mut typing) = self {
            if let Some(typing) = typing.take() {
                let _r = typing.stop();
            }
        }
    }
}

// impl Drop for MyTyping {
//     fn drop(&mut self) {
//         if let MyTyping::Typing(mut typing) = self {
//             if let Some(typing) = typing.take() {
//                 let _r = typing.stop();
//             }
//         }
//     }
// }
