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

use crate::utils::{user::Linked, IsacError};

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
}

impl Data {
    fn new() -> Self {
        Data {
            client: reqwest::Client::new(),
            patron: Arc::new(RwLock::new(vec![])),
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
        poise::FrameworkError::Command { error, ctx } => {
            if let Some(isac_error) = error.downcast_ref::<IsacError>() {
                let msg = match isac_error {
                    IsacError::LackOfArguments => {
                        "Click the button to check commands' usage and examples".to_string()
                    }
                    IsacError::UserNotLinked { msg } => msg.clone(),
                    IsacError::TooShortIgn { ign } => {
                        format!("❌ At least 3 charactars for ign searching: `{ign}`")
                    }
                    IsacError::APIError { msg } => format!("❌ WG API error: **{msg}**"),
                    IsacError::InvalidIgn { ign } => format!("❌ Invalid ign: `{ign}`"),
                    IsacError::PlayerIgnNotFound { ign, region } => {
                        format!("Player: `{ign}` not found in `{region}`")
                    }
                    IsacError::PlayerHidden { ign } => {
                        format!("Player `{ign}`'s profile is hidden.")
                    }
                    IsacError::PlayerNoBattle { ign } => {
                        format!("Player `{ign}` hasn't played any battle.")
                    }
                    IsacError::Cancelled => {
                        return ();
                    }
                    IsacError::UnkownError(err) => {
                        wws_error(&ctx, err).await;
                        return ();
                    }
                };
                let _r = ctx.reply(msg).await;
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
            if let Err(e) = poise::builtins::on_error(error).await {
                error!("Error while handling error: {}", e)
            }
        }
    }
}

async fn wws_error(ctx: &Context<'_>, error: &Error) {
    let user = ctx.author();
    let user_id = user.id;
    let channel_id = ctx.channel_id();
    let guild = ctx.guild().map(|f| f.name).unwrap_or("PM".to_string());
    let input = ctx.invocation_string();
    println!("ERROR \n[{input}] \n{user}, {user_id} \n{channel_id} \n{guild}");
}
