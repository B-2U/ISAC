use std::collections::HashMap;

use poise::{
    async_trait,
    serenity_prelude::{ArgumentConvert, ChannelId, Context, GuildId, User, UserParseError},
};

use crate::utils::{
    structs::{Linked, PartialPlayer},
    LoadSaveFromJson,
};

#[async_trait]
/// will only convert by user_id or mention, but not username
pub trait UserAddon: Sized {
    /// will only convert by user_id or mention, but not username
    #[must_use]
    async fn convert_strict(
        ctx: &Context,
        guild_id: Option<GuildId>,
        channel_id: Option<ChannelId>,
        s: &str,
    ) -> Result<User, UserParseError> {
        if s.chars().all(|c| c.is_ascii_digit()) || s.chars().any(|c| ['<', '@', '>'].contains(&c))
        {
            User::convert(ctx, guild_id, channel_id, s).await
        } else {
            Err(UserParseError::NotFoundOrMalformed)
        }
    }

    /// get the user's linked account if exist
    async fn get_player(&self, ctx: &crate::Context<'_>) -> Option<PartialPlayer> {
        let mut linked_js: HashMap<_, _> = Linked::load_json().await.into();
        linked_js.remove(&ctx.author().id)
    }
}
#[async_trait]
impl UserAddon for User {}
