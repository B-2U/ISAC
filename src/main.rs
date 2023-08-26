mod cmds;
mod tasks;
use cmds::*;
mod dc_utils;
mod utils;

use parking_lot::RwLock;
use poise::serenity_prelude::{self as serenity, Activity, UserId, Webhook};
use std::{
    collections::HashSet,
    env,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tracing::{debug, error};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use utils::{IsacHelp, IsacInfo};

use crate::{
    tasks::launch_renderer,
    utils::{
        structs::{ExpectedJs, GuildDefaultRegion, Linked, LittleConstant, Patrons, ShipsPara},
        IsacError, LoadSaveFromJson,
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
            owner::clan_season(),
            tools::roulette(),
            tools::rename(),
            tools::map(),
            tools::code(),
            tools::uid(),
            tools::clanuid(),
            wws::wws_hybrid(),
            leaderboard::top_hybrid(),
            setting::link_hybrid(),
            setting::wows_region(),
            patreon::background(),
            clan::clan_hybrid(),
            clan_top::clan_top(),
            recent::recent_hybrid(),
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
    // QA how to gracefully shut down?
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
        // QA gracfully?
        tokio::time::sleep(Duration::from_secs(3)).await;
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
    patron: Arc<RwLock<Patrons>>,
    expected_js: Arc<RwLock<ExpectedJs>>,
    ship_js: RwLock<ShipsPara>, // TODO make a command for update it
    link_js: RwLock<Linked>,
    wg_api_token: String,
    guild_default: RwLock<GuildDefaultRegion>,
    constant: RwLock<LittleConstant>,
}

impl Data {
    async fn new() -> Self {
        Data {
            client: reqwest::Client::new(),
            patron: Arc::new(RwLock::new(Patrons::default())),
            expected_js: Arc::new(RwLock::new(ExpectedJs::load_json_sync())),
            ship_js: RwLock::new(ShipsPara::load_json_sync()),
            link_js: RwLock::new(Linked::load_json_sync()),
            wg_api_token: env::var("WG_API").expect("Missing WG_API TOKEN"),
            guild_default: RwLock::new(GuildDefaultRegion::load_json_sync()),
            constant: RwLock::new(LittleConstant::load_json_sync()),
        }
    }
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
            // errors in commands here, include discord shits
            if let Some(isac_err) = error.downcast_ref::<IsacError>() {
                isac_error_handler(&ctx, isac_err).await;
            } else {
                wws_error_logging(&ctx, &error).await;
                error!("Error in command `{}`: {:?}", ctx.command().name, error,);
            }
        }
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
        error => {
            // panics in commands here
            if let Some(ctx) = error.ctx() {
                // thread 'tokio-runtime-worker' panicked at 'uuuuuuh', src\cmds\owner.rs:8:5
                // note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
                // QA 這種是rust底層的logging嗎 有沒有可能拿出來
                help_buttons_msg(&ctx, OOPS).await;
                wws_error_logging(&ctx, &error.to_string().into()).await;
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
                IsacInfo::UserNotLinked { user_name } => match user_name.as_ref() {
                    Some(user_name) => {
                        format!("**{user_name}** haven't linked to any wows account yet")
                    }
                    None => "You haven't linked your account yet.\nEnter `/link`".to_string(),
                },
                IsacInfo::TooShortIgn { ign } => {
                    format!("❌ At least 3 charactars for ign searching: `{ign}`")
                }
                IsacInfo::APIError { msg } => format!("❌ API error: `{msg}`"),
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
                    mode,
                } => {
                    format!(
                        "Player: `{ign}` hasn't played any battle in `{ship_name}` in `{}`",
                        mode.upper()
                    )
                }
                IsacInfo::AutoCompleteError => {
                    "❌ please select an option in the results!".to_string()
                }
                IsacInfo::ClanNoBattle { clan, season } => format!(
                    "**[{}]** ({}) did not participate in season {}",
                    clan.tag.replace("_", r"\_"),
                    clan.region,
                    season
                ),
                IsacInfo::NeedPremium { msg } => format!("{msg}\n{PREMIUM}"),
                IsacInfo::EmbedPermission => format!("❌ This error means ISAC don't have to permission to send embed here, please check the **Embed Links** in the permission setting, \nOr you can just re-invite ISAC in discord to let it grant the permission"),
            };
            let _r = ctx
                .send(|b| b.content(msg).reply(true).ephemeral(true))
                .await;
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
    let web_hook = Webhook::from_url(
        ctx,
        env::var("ERR_WEB_HOOK")
            .expect("Missing web hook url")
            .as_ref(),
    )
    .await
    .unwrap();
    let _r = web_hook
        .execute(ctx, false, |b| {
            b.content(format!(
                "``` ERROR \n[{input}] \n{user}, {user_id} \n{channel_id} \n{guild} ``` ``` {error} ```"
            ))
        })
        .await;
    error!("[{input}] \n{user}, {user_id} \n{channel_id} \n{guild} \n{error}");
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
const OOPS: &str = "***Oops! Something went wrong!***
click the **Document** button to check the commands usage
If this error keep coming out, please join our support server to report it
";

const PREMIUM: &str =
    "Seems you haven't join our Patreon, or link your discord account on Patreon yet :(
If you do like ISAC, [take a look?]( https://www.patreon.com/ISAC_bot )";
