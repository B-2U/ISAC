use poise::{
    async_trait,
    serenity_prelude::{ArgumentConvert, ChannelId, Context, GuildId, User, UserParseError},
};

#[async_trait]
/// will only convert by user_id or mention, but not username
pub trait UserAddon: Sized {
    type Err;
    async fn convert_strict(
        ctx: &Context,
        guild_id: Option<GuildId>,
        channel_id: Option<ChannelId>,
        s: &str,
    ) -> Result<Self, Self::Err>;
}
#[async_trait]
impl UserAddon for User {
    type Err = UserParseError;

    /// will only convert by user_id or mention, but not username
    #[must_use]
    async fn convert_strict(
        ctx: &Context,
        guild_id: Option<GuildId>,
        channel_id: Option<ChannelId>,
        s: &str,
    ) -> Result<Self, Self::Err> {
        if s.chars().all(|c| c.is_ascii_digit()) || s.chars().any(|c| ['<', '@', '>'].contains(&c)) {
            User::convert(ctx, guild_id, channel_id, s).await
        } else {
            Err(UserParseError::NotFoundOrMalformed)
        }
    }
}
