use std::sync::Arc;

use crate::Context;
use poise::serenity_prelude::Typing;

pub trait ContextAddon {
    async fn typing(&self) -> MyTyping;
}

/// a trait for `reply`
impl ContextAddon for Context<'_> {
    async fn typing(&self) -> MyTyping {
        match self {
            Context::Prefix(prefix_ctx) => {
                let typing = Typing::start(
                    Arc::clone(&prefix_ctx.serenity_context.http),
                    self.channel_id(),
                );
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
    Typing(Typing),
    Thinking,
}

impl MyTyping {
    pub fn stop(self) {
        if let MyTyping::Typing(typing) = self {
            typing.stop();
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
