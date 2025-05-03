use crate::dc_utils::{Args, ContextAddon};
use crate::utils::LoadSaveFromJson;
use crate::utils::wws_api::WowsApi;
use crate::{Context, Error};
use itertools::Itertools;
use poise::CreateReply;
use poise::serenity_prelude::{ArgumentConvert, Channel, CreateAllowedMentions, ReactionType};

#[poise::command(prefix_command, owners_only, hide_in_help)]
pub async fn test(_ctx: Context<'_>, #[rest] _args: Args) -> Result<(), Error> {
    Ok(())
}

#[poise::command(prefix_command, owners_only, hide_in_help)]
pub async fn cache_size(ctx: Context<'_>) -> Result<(), Error> {
    let size = std::mem::size_of_val(&ctx.data().cache);
    let _a = ctx.reply(format!("cache size: {} bytes", size)).await?;
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
pub async fn update_src(ctx: Context<'_>) -> Result<(), Error> {
    let _typing = ctx.typing().await;
    let api = WowsApi::new(&ctx);

    let res = api.encyclopedia_vehicles().await?;
    res.save_json().await;
    *ctx.data().ships.write() = res;
    ctx.reply("Updated").await?;
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
    let len = ctx.data().link.read().await.0.len();
    let _a = ctx.reply(len.to_string()).await?;
    Ok(())
}

#[poise::command(prefix_command, owners_only, hide_in_help)]
pub async fn who(ctx: Context<'_>, #[rest] mut args: Args) -> Result<(), Error> {
    let typing = ctx.typing().await;
    let partial_player = args.parse_user(&ctx).await?;
    typing.stop();
    let linked_users = {
        let rwg = ctx.data().link.read().await;
        rwg.0
            .iter()
            .filter(|(_, v)| **v == partial_player)
            .map(|(k, _)| k.clone())
            .collect::<Vec<_>>()
    };
    let output_str = linked_users
        .into_iter()
        .map(|user_id| format!("<@{user_id}>"))
        .join("\n");
    let _a = ctx
        .send(
            CreateReply::default()
                .content(output_str)
                .allowed_mentions(CreateAllowedMentions::default()),
        )
        .await?;
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
