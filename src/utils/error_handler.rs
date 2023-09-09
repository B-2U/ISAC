use std::env;
use tracing::error;

use poise::serenity_prelude::{self as serenity, CreateComponents, Webhook};

use crate::{
    dc_utils::CreateReplyAddon,
    utils::{self, IsacError},
    Context, Error,
};

// TODO: might need to be moved to a file for consts
const OOPS: &str = "***Oops! Something went wrong!***
click the **Document** button to check the commands usage
If this error keep coming out, please join our support server to report it
";

pub async fn isac_err_handler(ctx: &Context<'_>, error: &IsacError) {
    match error {
        IsacError::Help(help) => {
            let msg = match help {
                utils::IsacHelp::LackOfArguments => {
                    "Click the button to check commands' usage and examples".to_string()
                }
            };
            isac_get_help(ctx, Some(msg.as_ref())).await;
        }
        IsacError::Info(info) => {
            let _r = ctx
                .send(|b| b.content(info.to_string()).reply(true).ephemeral(true))
                .await;
        }
        IsacError::Cancelled => (),
        IsacError::UnknownError(err) => {
            isac_err_logging(&ctx, err).await;
            isac_get_help(&ctx, None).await
        }
    };
}

// TODO: better error msg, python's tracback?
/// logging to the terminal and discord channel
pub async fn isac_err_logging(ctx: &Context<'_>, error: &Error) {
    let user: &str = ctx.author().name.as_ref();
    let user_id = ctx.author().id;
    let channel_id = ctx.channel_id();
    let guild = ctx.guild().map(|f| f.name).unwrap_or("PM".to_string());
    let input = ctx.invocation_string();
    if let Ok(webhook_url) = env::var("ERR_WEB_HOOK") {
        let web_hook = Webhook::from_url(ctx, &webhook_url).await.unwrap();
        let _r = web_hook
            .execute(ctx, false, |b| {
                b.content(format!(
                    "``` ERROR \n[{input}] \n{user}, {user_id} \n{channel_id} \n{guild} ``` ``` {error} ```"
                ))
            })
            .await;
    }

    error!("[{input}] \n{user}, {user_id} \n{channel_id} \n{guild} \n{error}");
}

pub async fn isac_get_help(ctx: &Context<'_>, msg: Option<&str>) {
    let msg = match msg {
        Some(msg) => msg,
        None => OOPS,
    };
    let _r = ctx
        .send(|b| {
            b.content(msg)
                .set_components(help_view())
                .reply(true)
                .ephemeral(true)
        })
        .await;
}

fn help_view() -> CreateComponents {
    let mut view = CreateComponents::default();
    view.create_action_row(|r| {
        r.create_button(|b| {
            b.label("Document")
                .url("https://github.com/B-2U/ISAC")
                .style(serenity::ButtonStyle::Link)
        })
        .create_button(|b| {
            b.label("Support server")
                .url("https://discord.com/invite/z6sV6kEZGV")
                .style(serenity::ButtonStyle::Link)
        })
    });
    view
}
