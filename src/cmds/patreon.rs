use std::borrow::Cow;

use crate::{
    cmds::wws::func_wws,
    dc_utils::{ContextAddon, UserAddon},
    utils::{structs::PfpData, IsacError, IsacInfo, LoadSaveFromJson},
    Context, Error,
};
use poise::{
    self,
    serenity_prelude::{Attachment, AttachmentType, ChannelId},
};

/// Patreon feature, upload your custom profile background
#[poise::command(slash_command)]
pub async fn background(
    ctx: Context<'_>,
    #[description = "width : height ~ 4.5 : 1"] file: Attachment, // the String is a Serialized PartialPlayer struct
) -> Result<(), Error> {
    if !ctx.data().patron.read().check_user(&ctx.author().id) {
        Err(IsacError::Info(IsacInfo::NeedPremium {
            msg: "".to_string(),
        }))?
    }
    // QA shorter way to deal with this? as_ref()
    if !file
        .content_type
        .as_ref()
        .map(|s| s.contains("image"))
        .unwrap_or_default()
    {
        Err(IsacError::Info(IsacInfo::GeneralError {
            msg: "It's not a image!".to_string(),
        }))?
    }
    let player = ctx
        .author()
        .get_player(&ctx)
        .await
        .ok_or(IsacError::Info(IsacInfo::UserNotLinked { user_name: None }))?;
    let _typing = ctx.typing().await;
    // # isac-pfbg
    let img_byte = file.download().await?;
    let att: AttachmentType<'_> = AttachmentType::Bytes {
        data: Cow::Borrowed(&img_byte),
        filename: file.filename,
    };
    let channel = ChannelId(1141973121352618074);
    let msg = channel
        .send_message(ctx, |b| {
            b.add_file(att).content(format!(
                "{}, width: {} height: {}, size: {} KB",
                ctx.author(),
                file.width.unwrap_or_default(),
                file.height.unwrap_or_default(),
                file.size / 1000
            ))
        })
        .await?;
    let url = msg.attachments[0].url.clone();
    {
        let mut pfp_js = ctx.data().pfp.write();
        pfp_js
            .0
            .retain(|_, patron| patron.discord_id != ctx.author().id);
        pfp_js.0.insert(
            player.uid,
            PfpData {
                url,
                name: ctx.author().name.clone(),
                discord_id: ctx.author().id,
            },
        );
        pfp_js.save_json_sync();
    }

    func_wws(&ctx, player).await
}
