use crate::{
    dc_utils::{auto_complete, ContextAddon, UserAddon},
    utils::{
        structs::{Linked, PartialPlayer, Region},
        IsacError, IsacInfo, LoadSaveFromJson,
    },
    Context, Error,
};
use poise;

/// link your wows account
#[poise::command(slash_command, prefix_command)]
pub async fn link(
    ctx: Context<'_>,
    #[autocomplete = "auto_complete::player"] player: String, // the String is a Serialized PartialPlayer struct
) -> Result<(), Error> {
    let Ok(partial_player) = serde_json::from_str::<PartialPlayer>(&player) else {
        Err(IsacError::Info(IsacInfo::AutoCompleteError))?
    };
    let player = partial_player.get_player(&ctx).await?;
    let mut linked_js = Linked::load_json().await;
    linked_js.0.insert(ctx.author().id, partial_player);
    linked_js.save_json().await;
    let _r = ctx
        .reply(format!(
            "Successfully linked with `{}` ({})!",
            player.ign, player.region
        ))
        .await;
    Ok(())
}

/// Check / Set the default WoWs region for this server
#[poise::command(slash_command, rename = "wows-region")]
pub async fn wows_region(ctx: Context<'_>, region: Option<Region>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or(IsacError::Info(IsacInfo::GeneralError {
            msg: "❌ You have to use this command in a server".to_string(),
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
                let mut guard = ctx.data().guild_default.write();
                guard.0.insert(guild_id, region);
                // QA async not possible here right? since its in a RWlock
                guard.save_json_sync();
            }
            let _r = ctx
                .reply(format!("Default region set to **{region}** successfully!"))
                .await;
        } else {
            Err(IsacError::Info(IsacInfo::GeneralError {
                msg: "❌ You need admin permission to do this".to_string(),
            }))?
        }
    } else {
        let guild_default = {
            let guard = ctx.data().guild_default.read();
            guard
                .0
                .get(
                    &ctx.guild_id()
                        .unwrap_or_else(|| unreachable!("it should be dealed aboved")),
                )
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
