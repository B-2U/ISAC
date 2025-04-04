use crate::{
    Context, Data, Error,
    dc_utils::{UserAddon, autocomplete},
    structs::{AutocompletePlayer, Region},
    utils::{IsacError, IsacInfo, LoadSaveFromJson, wws_api::WowsApi},
};
use poise;

pub fn link_hybrid() -> poise::Command<Data, Error> {
    poise::Command {
        prefix_action: link_prefix().prefix_action,
        slash_action: link().slash_action,
        ..link()
    }
}

/// Link your wows account
#[poise::command(slash_command)]
pub async fn link(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete::player"]
    #[description = "your game server & ign"]
    player: AutocompletePlayer, // the String is a Serialized PartialPlayer struct
) -> Result<(), Error> {
    let api = WowsApi::new(&ctx);
    let partial_player = { player.fetch_partial_player(&api).await? };
    let player = partial_player.full_player(&api).await?;
    {
        let mut guard = ctx.data().link.write().await;
        guard.0.insert(ctx.author().id, partial_player);
        guard.save_json().await;
    }
    let _r = ctx
        .reply(format!(
            "Successfully linked with `{}` ({})!",
            player.ign, player.region
        ))
        .await;
    Ok(())
}

/// this is just a placeholder-like function telling user to use slash
#[poise::command(prefix_command, discard_spare_arguments)]
pub async fn link_prefix(ctx: Context<'_>) -> Result<(), Error> {
    let _r = ctx.reply("please use `/link` instead").await;
    Ok(())
}

/// Check / Set the default WoWs region for this server
#[poise::command(slash_command, rename = "wows-region")]
pub async fn wows_region(ctx: Context<'_>, region: Option<Region>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or(IsacError::Info(IsacInfo::GeneralError {
            msg: "You have to use this command in a server".to_string(),
        }))?;
    if let Some(region) = region {
        let is_admin = ctx
            .author()
            .get_permissions(&ctx)
            .await
            .map(|p| p.administrator());
        if let Ok(true) = is_admin {
            // block for RWlock
            {
                let mut guard = ctx.data().guild_default.write().await;
                guard.0.insert(guild_id, region);
                guard.save_json().await;
            }
            let _r = ctx
                .reply(format!("Default region set to **{region}** successfully!"))
                .await;
        } else {
            Err(IsacError::Info(IsacInfo::GeneralError {
                msg: "You need admin permission to do this".to_string(),
            }))?
        }
    } else {
        let guild_default = {
            let guard = ctx.data().guild_default.read().await;
            guard
                .0
                .get(&ctx.guild_id().expect("it should be dealed aboved"))
                .copied()
                .unwrap_or_default()
        };
        let _r = ctx
            .reply(format!(
                "The default region in this server is **{guild_default}**"
            ))
            .await;
    }

    Ok(())
}
