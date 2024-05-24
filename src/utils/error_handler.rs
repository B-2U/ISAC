use std::{
    env,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{debug, error};

use poise::serenity_prelude::{self as serenity, CreateComponents, Webhook};

use crate::{
    dc_utils::CreateReplyAddon,
    utils::{self, IsacError, IsacHelp},
    Context, Data, Error,
};

// TODO: might need to be moved to a file for consts
const OOPS: &str = "***Oops! Something went wrong!***
click the **Document** button to check the commands usage
If this error keep coming out, please join our support server to report it
";

pub async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::NotAnOwner { ctx: _ } => {}
        poise::FrameworkError::CooldownHit {
            remaining_cooldown,
            ctx,
        } => {
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            let msg = format!(
                "Command in cooldown, <t:{}:R>",
                (timestamp + remaining_cooldown).as_secs()
            );
            let _ = ctx.send(|builder| builder.content(msg).reply(true)).await;
        }
        poise::FrameworkError::ArgumentParse {
            error: _,
            input: _,
            ctx,
        } => isac_err_handler(&ctx, &IsacHelp::LackOfArguments.into()).await,
        // make the error become `debug` from `warning`
        poise::FrameworkError::UnknownCommand {
            ctx: _,
            msg: _,
            prefix,
            msg_content,
            framework: _,
            invocation_data: _,
            trigger: _,
        } => {
            debug!(
            "Recognized prefix `{prefix}`, but didn't recognize command name in `{msg_content}`")
        }

        poise::FrameworkError::Command { error, ctx } => {
            // errors returned here, include discord shits
            if let Some(isac_err) = error.downcast_ref::<IsacError>() {
                isac_err_handler(&ctx, isac_err).await;
            } else if let Some(serenity_err) = error.downcast_ref::<serenity::Error>() {
                error!(
                    "Error in command `{}`: {:?}",
                    ctx.command().name,
                    serenity_err
                );
            } else {
                isac_err_logging(&ctx, &error).await;
                error!("Error in command `{}`: {:?}", ctx.command().name, error,);
            }
        }
        poise::FrameworkError::CommandPanic { payload, ctx } => {
            isac_get_help(&ctx, None).await;
            isac_err_logging(
                &ctx,
                &payload.unwrap_or("No panic payload".to_string()).into(),
            )
            .await;
        }

        error => {
            // panics and else here
            if let Some(ctx) = error.ctx() {
                // thread 'tokio-runtime-worker' panicked at 'uuuuuuh', src\cmds\owner.rs:8:5
                // note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
                // QA 這種是rust底層的logging嗎 有沒有可能拿出來
                isac_get_help(&ctx, None).await;
                isac_err_logging(&ctx, &error.to_string().into()).await;
            } else if let Err(e) = poise::builtins::on_error(error).await {
                error!("Error while handling error: {}", e)
            }
        }
    }
}

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
            isac_err_logging(ctx, err).await;
            isac_get_help(ctx, None).await
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
