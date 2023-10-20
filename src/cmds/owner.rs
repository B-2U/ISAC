use crate::dc_utils::Args;
use crate::utils::LoadSaveFromJson;
use crate::{Context, Error};
use poise::serenity_prelude::{ArgumentConvert, Channel, ReactionType};

#[poise::command(prefix_command, owners_only, hide_in_help)]
pub async fn test(_ctx: Context<'_>, #[rest] _args: Args) -> Result<(), Error> {
    Ok(())
}

#[poise::command(prefix_command, owners_only, hide_in_help)]
pub async fn clan_season(ctx: Context<'_>, season: u32) -> Result<(), Error> {
    {
        ctx.data().constant.write().clan_season = season;
        ctx.data().constant.write().save_json_sync();
    }
    ctx.reply(format!("current clan season is {season} now!"))
        .await?;
    Ok(())
}

#[poise::command(prefix_command, owners_only, hide_in_help)]
pub async fn guilds(ctx: Context<'_>) -> Result<(), Error> {
    let _cache = ctx.cache();
    ctx.reply(_cache.guilds().len().to_string()).await?;
    Ok(())
}

#[poise::command(prefix_command, owners_only, hide_in_help)]
pub async fn users(ctx: Context<'_>) -> Result<(), Error> {
    let len = ctx.data().link_js.read().0.len();
    let _a = ctx.reply(len.to_string()).await?;
    Ok(())
}

#[poise::command(prefix_command, owners_only, hide_in_help)]
pub async fn send(ctx: Context<'_>, channel_id: String, #[rest] msg: String) -> Result<(), Error> {
    let Context::Prefix(prefix_ctx) = ctx else {
        Err("not a prefix context!")?
    };
    let channel = Channel::convert(
        ctx.serenity_context(),
        ctx.guild_id(),
        Some(ctx.channel_id()),
        &channel_id,
    )
    .await;
    let result_emoji = match channel {
        Ok(channel) => {
            channel.id().say(ctx, msg).await?;
            "✅".to_string()
        }
        Err(_) => "❌".to_string(),
    };
    prefix_ctx
        .msg
        .react(ctx, ReactionType::Unicode(result_emoji))
        .await?;
    Ok(())
}
