mod cmds;
use cmds::*;
mod dc_utils;
mod utils;

use poise::serenity_prelude::{self as serenity, Activity, UserId};
use std::{
    collections::HashSet,
    env,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::error;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

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
    let bot = poise::Framework::builder()
        .token(token)
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", _ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                ctx.set_activity(Activity::listening(".help")).await;
                Ok(Data::new())
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
    // this is how to use serenity's `data`
    // {
    //     let mut data = bot.client().data.write().await;
    //     data.insert::<ReqClient>(reqwest::Client::new());
    // }
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    if let Err(why) = bot.start().await {
        error!("Client error: {:?}", why);
    }
}

/// My custom Data
#[allow(dead_code)]
pub struct Data {
    client: reqwest::Client,
}

impl Data {
    fn new() -> Self {
        Data {
            client: reqwest::Client::new(),
        }
    }
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
            error!("Error in command `{}`: {:?}", ctx.command().name, error,);
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

// #[derive(Serialize, Deserialize, Debug, Default)]
// #[allow(dead_code)]
// pub struct Setu {
//     imgs: Vec<String>,
// }
// impl Setu {
//     fn load<P: AsRef<Path> + std::fmt::Display>(path: P) -> Self {
//         load_json(&path).unwrap_or_else(|err| {
//             error!("{path} loading failed, use dafault. reason:\n{err}");
//             Self::default()
//         })
//     }
// }
