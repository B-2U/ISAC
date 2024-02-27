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
use tokio::io::AsyncWriteExt;

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
    let file_type = match &file.content_type {
        Some(content_type) if content_type.contains("image") => content_type
            .split('/')
            .nth(1)
            .ok_or(IsacError::Info(IsacInfo::GeneralError {
                msg: "failed to get image type!".to_string(),
            }))
            .map(|s| s.to_string()),
        _ => Err(IsacError::Info(IsacInfo::GeneralError {
            msg: "It's not an image!".to_string(),
        })),
    }?;

    let player = ctx
        .author()
        .get_player(&ctx)
        .await
        .ok_or(IsacError::Info(IsacInfo::UserNotLinked { user_name: None }))?;
    let _typing = ctx.typing().await;
    // # isac-pfbg
    let img_byte = file.download().await?;

    let file_path = format!("./user_data/pfp/{}.{}", ctx.author().id, file_type);
    let mut fs = tokio::fs::File::create(&file_path)
        .await
        .unwrap_or_else(|err| panic!("failed to create file: {:?}, Err: {err}", file_path));

    if let Err(err) = fs.write_all(&img_byte).await {
        panic!("Failed to write JSON to file: {:?}. Err: {err}", file_path);
    }

    let att: AttachmentType<'_> = AttachmentType::Bytes {
        data: Cow::Borrowed(&img_byte),
        filename: file.filename,
    };
    let channel = ChannelId(1141973121352618074);
    let _msg = channel
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
    {
        let mut pfp_js = ctx.data().pfp.write().await;
        pfp_js
            .0
            .retain(|_, patron| patron.discord_id != ctx.author().id);
        pfp_js.0.insert(
            player.uid,
            PfpData {
                url: file_path,
                name: ctx.author().name.clone(),
                discord_id: ctx.author().id,
            },
        );
        pfp_js.save_json().await;
    }

    func_wws(&ctx, player).await
}
