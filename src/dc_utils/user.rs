use poise::serenity_prelude::{
    ArgumentConvert, CacheHttp, ChannelId, Context, GuildId, Permissions, User,
};

use crate::{
    structs::PartialPlayer,
    utils::{IsacError, IsacInfo},
    Error,
};

pub trait UserAddon: Sized {
    /// will only convert by user_id or mention, but not username
    #[must_use]
    async fn convert_strict(
        ctx: &Context,
        guild_id: Option<GuildId>,
        channel_id: Option<ChannelId>,
        s: &str,
    ) -> Result<User, IsacError>;

    /// get the user's linked account if exist
    async fn get_player(&self, ctx: &crate::Context<'_>) -> Option<PartialPlayer>;

    /// get permissions
    async fn get_permissions(&self, ctx: &crate::Context<'_>) -> Result<Permissions, Error>;
}

impl UserAddon for User {
    async fn convert_strict(
        ctx: &Context,
        guild_id: Option<GuildId>,
        channel_id: Option<ChannelId>,
        s: &str,
    ) -> Result<User, IsacError> {
        let error_message = IsacInfo::GeneralError {
            msg: format!("`{s}` is not a valid user, please enter a pign like <@930855839961591849> or a Discord User ID"),
        };

        if s.chars().any(|c| matches!(c, '<' | '@' | '>')) || s.chars().all(|c| c.is_ascii_digit())
        {
            User::convert(ctx, guild_id, channel_id, s)
                .await
                .map_err(|_| error_message.into())
        } else {
            Err(error_message.into())
        }
    }

    /// get the user's linked account if exist
    async fn get_player(&self, ctx: &crate::Context<'_>) -> Option<PartialPlayer> {
        ctx.data().link.read().await.get(&self.id)
    }

    /// get permissions
    async fn get_permissions(&self, ctx: &crate::Context<'_>) -> Result<Permissions, Error> {
        let guild_id = ctx.guild_id().ok_or::<Error>("Not in a guild".into())?;
        ctx.http()
            .get_member(guild_id, ctx.author().id)
            .await?
            .permissions(ctx.cache().ok_or::<Error>("get cache failed".into())?)
            .map_err(|err| err.into())
    }
}
