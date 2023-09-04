mod cmds;
use cmds::*;
mod dc_utils;
mod tasks;
mod template_data;
mod utils;

use parking_lot::RwLock;
use poise::serenity_prelude::{self as serenity, Activity, UserId};
use std::{
    collections::HashSet,
    env,
    ops::Deref,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tracing::{debug, error};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::{
    tasks::launch_renderer,
    utils::{
        error_handler::{isac_err_handler, isac_err_logging, isac_get_help},
        structs::{
            ExpectedJs, GuildDefaultRegion, Linked, LittleConstant, Patrons, Pfp, ShipLeaderboard,
            ShipsPara,
        },
        IsacError, IsacHelp, LoadSaveFromJson,
    },
};

// Types used by all command functions
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_env("LOGGER"))
        .init();
    dotenv::dotenv().expect("Failed to load .env file, check .env.example!");
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
    let data = Data::new();
    let arc_data = data.clone();
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
    let patron = Arc::clone(&arc_data.patron);
    tokio::spawn(async move { tasks::patron_updater(http, patron).await });
    // update expected json
    let client = arc_data.client.clone();
    let expected = Arc::clone(&arc_data.expected_js);
    tokio::spawn(async move { tasks::expected_updater(client, expected).await });

    // this is how to use serenity's `data`
    // {
    //     let mut data = bot.client().data.write().await;
    //     data.insert::<ReqClient>(reqwest::Client::new());
    // }
    if let Err(why) = bot.start().await {
        error!("Client error: {:?}", why);
    }
}

/// My custom Data, it already uses an [`Arc`] internally.
/// well its a bit tricky, but I'm lazy to clone them before bot building one by one
#[derive(Clone)]
pub struct Data {
    pub inner: Arc<DataInner>,
}

impl Deref for Data {
    type Target = DataInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Data {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DataInner::new()),
        }
    }
}
pub struct DataInner {
    client: reqwest::Client,
    patron: Arc<RwLock<Patrons>>,
    expected_js: Arc<RwLock<ExpectedJs>>,
    ship_js: RwLock<ShipsPara>, // TODO make a command for update it
    link_js: RwLock<Linked>,
    wg_api_token: String,
    guild_default: RwLock<GuildDefaultRegion>,
    constant: RwLock<LittleConstant>,
    pfp: RwLock<Pfp>,
    leaderboard: Mutex<ShipLeaderboard>,
}

impl DataInner {
    fn new() -> Self {
        DataInner {
            client: reqwest::Client::new(),
            patron: Arc::new(RwLock::new(Patrons::default())),
            expected_js: Arc::new(RwLock::new(ExpectedJs::load_json_sync())),
            ship_js: RwLock::new(ShipsPara::load_json_sync()),
            link_js: RwLock::new(Linked::load_json_sync()),
            wg_api_token: env::var("WG_API").expect("Missing WG_API TOKEN"),
            guild_default: RwLock::new(GuildDefaultRegion::load_json_sync()),
            constant: RwLock::new(LittleConstant::load_json_sync()),
            pfp: RwLock::new(Pfp::load_json_sync()),
            leaderboard: Mutex::new(ShipLeaderboard::load_json_sync()),
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
        } => isac_err_handler(&ctx, &IsacHelp::LackOfArguments.into()).await,

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
            // panics and else here
            if let Some(ctx) = error.ctx() {
                // thread 'tokio-runtime-worker' panicked at 'uuuuuuh', src\cmds\owner.rs:8:5
                // note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
                // QA 這種是rust底層的logging嗎 有沒有可能拿出來
                isac_get_help(&ctx, None).await;
                isac_err_logging(&ctx, &error.to_string().into()).await;
            } else {
                if let Err(e) = poise::builtins::on_error(error).await {
                    error!("Error while handling error: {}", e)
                }
            }
        }
    }
}
