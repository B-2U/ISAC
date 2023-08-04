use crate::{
    dc_utils::{Args, ContextAddon},
    utils::PlayerCommon,
    Context, Error,
};

#[poise::command(prefix_command)]
pub async fn wws(ctx: Context<'_>, #[rest] args: Option<Args>) -> Result<(), Error> {
    let mut args = args.unwrap_or_default();
    let _typing = ctx.typing().await;
    let player = args.parse_user(&ctx).await?.get_player(&ctx).await?;
    let clan = player.clan(&ctx).await?;
    if args.is_empty() {
        // wws
    } else {
        // wws ship
    }

    Ok(())
}
