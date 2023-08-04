mod cmds;
use cmds::*;
mod dc_utils;
mod utils;

use dc_utils::ContextAddon;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use poise::serenity_prelude::{self as serenity, Activity, RoleId, UserId};
use std::{
    collections::{HashMap, HashSet},
    env,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tracing::error;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use utils::{IsacHelp, IsacInfo};

use crate::utils::{structs::Linked, IsacError};

// Types used by all command functions
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to load .env file");
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_env("LOGGER"))
        .init();

    let (prefix, token) = if cfg!(windows) {
        ("-", env::var("WIP_TOKEN").expect("Missing TOKEN"))
    } else {
        (".", env::var("TOKEN").expect("Missing TOKEN"))
    };

    let options = poise::FrameworkOptions {
        owners: HashSet::from([UserId(930855839961591849)]),
        commands: vec![
            general::invite(),
            general::help(),
            owner::guilds(),
            owner::test(),
            owner::send(),
            owner::users(),
            tools::roulette(),
            tools::rename(),
            tools::map(),
            tools::code(),
            tools::uid(),
            tools::clanuid(),
        ],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some(prefix.into()),
            mention_as_prefix: false,
            ..Default::default()
        },
        /// The global error handler for all error cases that may occur
        on_error: |error| Box::pin(on_error(error)),
        skip_checks_for_owners: true,
        ..Default::default()
    };
    let data = Data::new();
    let patrons_arc = Arc::clone(&data.patron);
    let bot = poise::Framework::builder()
        .token(token)
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", _ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                ctx.set_activity(Activity::listening(".help")).await;
                Ok(data)
            })
        })
        .options(options)
        .intents(
            serenity::GatewayIntents::non_privileged()
                | serenity::GatewayIntents::MESSAGE_CONTENT
                | serenity::GatewayIntents::GUILD_MEMBERS,
        )
        .build()
        .await
        .unwrap();
    let shard_manager = bot.client().shard_manager.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });
    // update patreon
    let http = bot.client().cache_and_http.http.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(180));
        static GUILD_ID: Lazy<u64> = Lazy::new(|| env::var("GUILD_ID").unwrap().parse().unwrap());
        static PATREON_ID: Lazy<RoleId> =
            Lazy::new(|| RoleId(env::var("PATREON_ROLE_ID").unwrap().parse().unwrap()));
        static SUP_ID: Lazy<RoleId> =
            Lazy::new(|| RoleId(env::var("SUPPORTER_ROLE_ID").unwrap().parse().unwrap()));
        loop {
            interval.tick().await;
            let linked_js: HashMap<_, _> = Linked::load().await.into();
            let guild = http.get_guild(*GUILD_ID).await.unwrap();
            let patrons = guild
                .members(&http, None, None)
                .await
                .unwrap()
                .into_iter()
                .filter(|m| m.roles.contains(&PATREON_ID) || m.roles.contains(&SUP_ID))
                .map(|m| Patron {
                    uid: linked_js
                        .get(&m.user.id)
                        .map(|linked_user| linked_user.uid)
                        .unwrap_or(0),
                    discord_id: m.user.id,
                })
                .collect::<Vec<_>>();
            *patrons_arc.write() = patrons;
        }
    });
    // this is how to use serenity's `data`
    // {
    //     let mut data = bot.client().data.write().await;
    //     data.insert::<ReqClient>(reqwest::Client::new());
    // }
    if let Err(why) = bot.start().await {
        error!("Client error: {:?}", why);
    }
}

/// My custom Data
pub struct Data {
    client: reqwest::Client,
    patron: Arc<RwLock<Vec<Patron>>>,
    wg_api_token: String,
}

impl Data {
    fn new() -> Self {
        Data {
            client: reqwest::Client::new(),
            patron: Arc::new(RwLock::new(vec![])),
            wg_api_token: env::var("WG_API").expect("Missing WG_API TOKEN"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Patron {
    discord_id: UserId,
    uid: u64,
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
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
        } => isac_error_handler(&ctx, &IsacHelp::LackOfArguments.into()).await,

        poise::FrameworkError::Command { error, ctx } => {
            if let Some(isac_err) = error.downcast_ref::<IsacError>() {
                isac_error_handler(&ctx, isac_err).await;
            } else {
                error!("Error in command `{}`: {:?}", ctx.command().name, error,);
            }
        }
        // make the error become `debug` from `warning`
        // poise::FrameworkError::UnknownCommand {
        //     ctx: _,
        //     msg: _,
        //     prefix,
        //     msg_content,
        //     framework: _,
        //     invocation_data: _,
        //     trigger: _,
        // } => {
        //     tracing::debug!(
        //     "Recognized prefix `{prefix}`, but didn't recognize command name in `{msg_content}`")
        // }
        error => {
            if let Some(ctx) = error.ctx() {
                help_buttons_msg(&ctx, OOPS).await;
            } else {
                if let Err(e) = poise::builtins::on_error(error).await {
                    error!("Error while handling error: {}", e)
                }
            }
        }
    }
}

async fn isac_error_handler(ctx: &Context<'_>, error: &IsacError) {
    match error {
        IsacError::Help(help) => {
            let msg = match help {
                utils::IsacHelp::LackOfArguments => {
                    "Click the button to check commands' usage and examples".to_string()
                }
            };
            help_buttons_msg(&ctx, msg).await;
        }
        IsacError::Info(info) => {
            let msg = match info {
                IsacInfo::UserNotLinked { msg } => msg.clone(),
                IsacInfo::TooShortIgn { ign } => {
                    format!("❌ At least 3 charactars for ign searching: `{ign}`")
                }
                IsacInfo::APIError { msg } => format!("❌ WG API error: **{msg}**"),
                IsacInfo::InvalidIgn { ign } => format!("❌ Invalid ign: `{ign}`"),
                IsacInfo::PlayerIgnNotFound { ign, region } => {
                    format!("Player: `{ign}` not found in `{region}`")
                }
                IsacInfo::PlayerHidden { ign } => {
                    format!("Player `{ign}`'s profile is hidden.")
                }
                IsacInfo::PlayerNoBattle { ign } => {
                    format!("Player `{ign}` hasn't played any battle.")
                }
                IsacInfo::GeneralError { msg } => msg.clone(),
                IsacInfo::InvalidClan { clan } => format!("❌ Invalid clan name: `{clan}`"),
                IsacInfo::ClanNotFound { clan, region } => {
                    format!("Clan: `{clan}` not found in `{region}`")
                }
            };
            let _r = ctx.reply(msg).await;
        }
        IsacError::Cancelled => (),
        IsacError::UnknownError(err) => {
            wws_error_logging(&ctx, err).await;
            help_buttons_msg(&ctx, OOPS).await;
        }
    };
}

// todo: better error msg, python's tracback?
/// loging to the terminal and discord channel
async fn wws_error_logging(ctx: &Context<'_>, error: &Error) {
    let user = ctx.author();
    let user_id = user.id;
    let channel_id = ctx.channel_id();
    let guild = ctx.guild().map(|f| f.name).unwrap_or("PM".to_string());
    let input = ctx.invocation_string();
    println!("ERROR \n[{input}] \n{user}, {user_id} \n{channel_id} \n{guild} \n{error}");
}

async fn help_buttons_msg(ctx: &Context<'_>, msg: impl AsRef<str>) {
    let result = ctx
        .send(|b| {
            b.components(|c| {
                c.create_action_row(|r| {
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
                })
            })
            .content(msg.as_ref())
            .reply(true)
        })
        .await;
    if let Err(err) = result {
        eprintln!("help_buttons_msg error: {err}");
    }
}

// todo: might need to be moved to a file for consts
const OOPS: &str = r#"***Oops! Something went wrong!***
click the `Document` button to check the doc
If this error keep coming out, please join our support server to report it
"#;
