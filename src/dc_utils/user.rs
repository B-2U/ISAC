use std::collections::HashMap;

use poise::{
    async_trait,
    serenity_prelude::{
        ArgumentConvert, CacheHttp, ChannelId, Context, GuildId, Permissions, User, UserParseError,
    },
};

use crate::{
    utils::{
        structs::{Linked, PartialPlayer},
        LoadSaveFromJson,
    },
    Error,
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

    /// get permissions
    async fn get_permissions(&self, ctx: &crate::Context<'_>) -> Result<Permissions, Error> {
        let guild_id = ctx.guild_id().ok_or::<Error>("Not in a guild".into())?;
        ctx.http()
            .get_member(guild_id.0, ctx.author().id.0)
            .await?
            .permissions(&ctx.cache().ok_or::<Error>("get cache failed".into())?)
            .map_err(|err| err.into())
    }
}
#[async_trait]
impl UserAddon for User {}
