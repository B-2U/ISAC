use std::borrow::Cow;

use crate::{
    cmds::wws::func_wws,
    dc_utils::{ContextAddon, UserAddon},
    utils::{
        structs::{Pfp, PfpData},
        IsacError, IsacInfo, LoadSaveFromJson,
    },
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
    #[description = "please crop the image to about 4.5 : 1"] file: Attachment, // the String is a Serialized PartialPlayer struct
) -> Result<(), Error> {
    if !ctx.data().patron.read().check_user(ctx.author().id) {
        Err(IsacError::Info(IsacInfo::GeneralError {
            msg: "Seems you haven't join our Patreon, or link your discord account on Patreon yet :( \nIf you do like ISAC, maybe? https://www.patreon.com/ISAC_bot".to_string(),
        }))?
    }
    // QA shorter way to deal with this? as_ref()
    if !file
        .content_type
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or_default()
        .contains("image")
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
    let mut pfp_js = Pfp::load_json().await;
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
    pfp_js.save_json().await;
    func_wws(&ctx, player).await
}
