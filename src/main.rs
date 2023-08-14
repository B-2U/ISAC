mod cmds;
mod tasks;
use cmds::*;
mod dc_utils;
mod utils;

use dc_utils::ContextAddon;
use parking_lot::RwLock;
use poise::serenity_prelude::{self as serenity, Activity, UserId};
use std::{
    collections::HashSet,
    env,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::error;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use utils::{IsacHelp, IsacInfo};

use crate::{
    tasks::launch_renderer,
    utils::{
        structs::{ExpectedJs, GuildDefaultRegion, ShipsPara},
        IsacError,
    },
};

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
    launch_renderer().await;

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
            wws::wws(),
            wws::wws_slash(),
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
    // TODO better way than cloning everything? seems the framwork doesn't accept Arc<Data>....
    let data = Data::new().await;
    let patrons_arc = Arc::clone(&data.patron);
    let expected_js_arc = Arc::clone(&data.expected_js);
    let client_clone = data.client.clone();
    let bot = poise::Framework::builder()
        .token(token)
        .setup(move |ctx, ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", ready.user.name);
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
    let shard_manager = Arc::clone(&bot.shard_manager());
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });
    // update patreon
    let http = bot.client().cache_and_http.http.clone();
    tokio::spawn(async move { tasks::patron_updater(http, patrons_arc).await });
    // update expected json

    tokio::spawn(async move { tasks::expected_updater(client_clone, expected_js_arc).await });

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
    expected_js: Arc<RwLock<ExpectedJs>>,
    ship_js: Arc<RwLock<ShipsPara>>,
    wg_api_token: String,
    guild_default: Arc<RwLock<GuildDefaultRegion>>, // browser: Arc<fantoccini::Client>,
}

impl Data {
    async fn new() -> Self {
        Data {
            client: reqwest::Client::new(),
            patron: Arc::new(RwLock::new(vec![])),
            expected_js: Arc::new(RwLock::new(ExpectedJs::new())),
            ship_js: Arc::new(RwLock::new(ShipsPara::new())),
            wg_api_token: env::var("WG_API").expect("Missing WG_API TOKEN"),
            guild_default: Arc::new(RwLock::new(GuildDefaultRegion::new())),
            // browser: Arc::new(
            //     fantoccini::ClientBuilder::native()
            //         .connect("http://localhost:4444")
            //         .await
            //         .expect("failed to connect to WebDriver"),
            // ),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Patron {
    discord_id: UserId,
    uid: u64,
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
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
                IsacInfo::APIError { msg } => format!("❌ API error: **{msg}**"),
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
                IsacInfo::ShipNotFound { ship_name } => format!("Warship: `{ship_name}` not found"),
                IsacInfo::PlayerNoBattleShip {
                    ign,
                    ship_name,
                    region,
                } => {
                    format!(
                        "Player: `{ign}` hasn't played any battle in `{ship_name}` in `{region}`"
                    )
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

// TODO: better error msg, python's tracback?
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

// TODO: might need to be moved to a file for consts
const OOPS: &str = r#"***Oops! Something went wrong!***
click the `Document` button to check the doc
If this error keep coming out, please join our support server to report it
"#;
