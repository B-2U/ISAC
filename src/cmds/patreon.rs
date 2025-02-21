use crate::{
    Context, Error,
    cmds::wws::func_wws,
    dc_utils::{ContextAddon, UserAddon},
    structs::BannerData,
    utils::{IsacError, IsacInfo, LoadSaveFromJson},
};
use poise::{
    self,
    serenity_prelude::{Attachment, ChannelId, CreateAttachment, CreateMessage},
};
use tokio::{fs, io::AsyncWriteExt};

/// Patreon feature, upload your custom profile background
#[poise::command(slash_command)]
pub async fn background(
    ctx: Context<'_>,
    #[description = "best resolution: 980*220px (4.45 : 1)"] file: Attachment, // the String is a Serialized PartialPlayer struct
) -> Result<(), Error> {
    if !ctx.data().patron.read().check_user(&ctx.author().id) {
        Err(IsacError::Info(IsacInfo::NeedPremium {
            msg: "".to_string(),
        }))?
    }
    let file_type = file
        .content_type
        .as_deref()
        .filter(|ct| ct.starts_with("image/"))
        .and_then(|ct| ct.split('/').nth(1))
        .ok_or_else(|| {
            IsacError::Info(IsacInfo::GeneralError {
                msg: "Invalid or non-image file type".to_string(),
            })
        })?
        .to_string();

    let player = ctx.author().get_player(&ctx).await?;
    let _typing = ctx.typing().await;
    // download banner
    let img_byte = file.download().await?;
    let file_path = format!("./user_data/banner/{}.{}", ctx.author().id, file_type);

    {
        let mut banner_js = ctx.data().banner.write().await;
        let keys_to_remove = banner_js
            .0
            .iter()
            .filter(|(_, value)| value.discord_id == ctx.author().id)
            .map(|(&key, _)| key)
            .collect::<Vec<_>>();
        for key in keys_to_remove {
            if let Some(v) = banner_js.0.remove(&key) {
                let _r = fs::remove_file(v.url).await;
            }
        }
        banner_js.0.insert(
            player.uid,
            BannerData {
                url: file_path.clone(),
                name: ctx.author().name.clone(),
                discord_id: ctx.author().id,
            },
        );
        banner_js.save_json().await;
    }
    // save the new banner
    let mut fs = tokio::fs::File::create(&file_path)
        .await
        .unwrap_or_else(|err| panic!("failed to create file: {:?}, Err: {err}", file_path));

    if let Err(err) = fs.write_all(&img_byte).await {
        panic!("Failed to write IMAGE to file: {:?}. Err: {err}", file_path);
    }
    // showing preview
    func_wws(&ctx, player).await?;

    // sending log
    let att = CreateAttachment::bytes(img_byte, file.filename);
    let channel = ChannelId::new(1141973121352618074);
    let _msg = channel
        .send_message(
            ctx,
            CreateMessage::default().add_file(att).content(format!(
                "{}, width: {} height: {}, size: {} KB",
                ctx.author(),
                file.width.unwrap_or_default(),
                file.height.unwrap_or_default(),
                file.size / 1000
            )),
        )
        .await?;
    Ok(())
}
