// Using enum unqualified is bad form. See https://youtu.be/8j_FbjiowvE?t=97.
#![deny(clippy::enum_glob_use)]

mod cmds;
use cmds::*;
mod dc_utils;
mod structs;
mod tasks;
mod template_data;
mod utils;

use futures::future::join_all;
use poise::serenity_prelude::{
    self as serenity, ActivityData, ClientBuilder, ExecuteWebhook, UserId, Webhook,
};
use std::{collections::HashSet, env, ops::Deref, sync::Arc};
use tracing::{error, info, warn};
use tracing_subscriber::{EnvFilter, prelude::*};

use crate::{
    structs::{
        Banner, ExpectedJs, GuildDefaultRegion, Linked, LittleConstant, Patrons, ShipLeaderboard,
        ShipsPara, user_search_history::SearchCache,
    },
    tasks::launch_renderer,
    utils::{LoadSaveFromJson, error_handler},
};

// Types used by all command functions
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to load .env file, check .env.example!");
    tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_env("LOGGER"))
        .finish()
        .init();

    let is_product = &env::var("IS_PRODUCT")
        .expect("Missing IS_PRODUCT")
        .to_lowercase()
        == "true";

    let (prefix, token) = if is_product {
        (".", env::var("TOKEN").expect("Missing TOKEN"))
    } else {
        warn!("IS_PRODUCT = false, using test bot token");
        ("-", env::var("WIP_TOKEN").expect("Missing WIP_TOKEN"))
    };

    let options = poise::FrameworkOptions {
        owners: HashSet::from([UserId::new(930855839961591849)]),
        commands: vec![
            general::invite(),
            general::help(),
            owner::guilds(),
            owner::test(),
            owner::cache_size(),
            owner::send(),
            owner::users(),
            owner::clan_season(),
            owner::update_src(),
            tools::roulette(),
            tools::history(),
            tools::map(),
            tools::ign(),
            tools::code(),
            tools::uid(),
            tools::clanuid(),
            wws::wws_hybrid(),
            top::top_hybrid(),
            setting::link_hybrid(),
            setting::wows_region(),
            patreon::background(),
            clan::clan_hybrid(),
            clan_top::clan_top(),
            recent::recent_hybrid(),
            server_top::server_top_hybrid(),
        ],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some(prefix.into()),
            mention_as_prefix: false,
            ..Default::default()
        },
        // The global error handler for all error cases that may occur
        on_error: |error| Box::pin(error_handler::on_error(error)),
        skip_checks_for_owners: true,
        ..Default::default()
    };
    let data = Data::new().await;
    let arc_data = data.clone();
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let mut bot = ClientBuilder::new(
        token,
        serenity::GatewayIntents::non_privileged()
            | serenity::GatewayIntents::MESSAGE_CONTENT
            | serenity::GatewayIntents::GUILD_MEMBERS,
    )
    .framework(
        poise::Framework::builder()
            .setup(move |ctx, ready, framework| {
                Box::pin(async move {
                    info!("Logged in as {}", ready.user.name);
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                    Ok(data)
                })
            })
            .options(options)
            .build(),
    )
    .activity(ActivityData::listening(".help"))
    .await
    .unwrap();
    let shard_manager = bot.shard_manager.clone();

    // ctrl_c catcher for both Win and Unix
    let tx2 = tx.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        info!("Ctrl C, ISAC shutting down...");
        let _ = tx2.send(());
        // QA gracfully?
    });
    // Unix SIGTERM catcher
    #[cfg(target_os = "linux")]
    {
        tokio::spawn(async move {
            let mut sig = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Could not register SIGTERM handler");
            sig.recv().await;
            info!("SIGTERM, ISAC shutting down...");
            let _ = tx.send(());
        });
    }

    // a webhook logger, send the received message to discord logging channel
    let webhook_http = bot.http.clone();
    let (webhook_tx, mut webhook_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    tokio::spawn(async move {
        let web_hook = Webhook::from_url(&webhook_http, &env::var("ERR_WEB_HOOK").unwrap())
            .await
            .unwrap();
        loop {
            while let Some(input) = webhook_rx.recv().await {
                let _r = web_hook
                    .execute(&webhook_http, false, ExecuteWebhook::new().content(input))
                    .await;
            }
        }
    });

    // update patreon
    let http = bot.http.clone();
    let patron = Arc::clone(&arc_data.patron);
    let webhook_tx_new = webhook_tx.clone();
    tokio::spawn(async move { tasks::patron_updater(http, patron, webhook_tx_new).await });
    // update ShipsPara
    let client = arc_data.client.clone();
    let ships = Arc::clone(&arc_data.ships);
    let webhook_tx_new = webhook_tx.clone();
    tokio::spawn(async move { tasks::ships_para_updater(client, ships, webhook_tx_new).await });
    // update expected json
    let client = arc_data.client.clone();
    let expected = Arc::clone(&arc_data.expected);
    let webhook_tx_new = webhook_tx.clone();

    tokio::spawn(async move { tasks::expected_updater(client, expected, webhook_tx_new).await });

    let _renderer = launch_renderer().await; // it's used in linux specific code below

    tokio::spawn(async move {
        if let Err(why) = bot.start().await {
            error!("Client error: {:?}", why);
        }
    });
    if rx.recv().is_err() {
        error!("All signal handlers hung up, shutting down...");
    }
    // cleaning up

    // send message to discord log channel
    if is_product {
        let _ = webhook_tx.send("Bot shutting down...".into());
    }
    // TODO: use async drop trait?
    let lb_mg = arc_data.leaderboard.lock().await;
    lb_mg.save_json().await;
    info!("Saved leaderboard.json");

    let cache_mg = arc_data.cache.lock().await;
    join_all(cache_mg.users.iter().map(|(_, data)| data.save())).await;
    info!("Saved users' history cache");

    // close renderer
    #[cfg(target_os = "linux")]
    if let Some(renderer_pid) = _renderer.id() {
        unsafe { libc::kill(renderer_pid as i32, libc::SIGINT) };
    }
    shard_manager.shutdown_all().await;
}

/// My custom Data, it already uses an [`Arc`] internally just like [reqwest::Client].
#[derive(Clone)]
pub struct Data {
    pub inner: Arc<DataInner>,
}
impl Data {
    async fn new() -> Self {
        Self {
            inner: Arc::new(DataInner::new().await),
        }
    }
}

impl Deref for Data {
    type Target = DataInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct DataInner {
    client: reqwest::Client,
    patron: Arc<parking_lot::RwLock<Patrons>>,
    expected: Arc<parking_lot::RwLock<ExpectedJs>>,
    ships: Arc<parking_lot::RwLock<ShipsPara>>,
    constant: parking_lot::RwLock<LittleConstant>,
    link: tokio::sync::RwLock<Linked>,
    wg_api_token: String,
    guild_default: tokio::sync::RwLock<GuildDefaultRegion>,
    banner: tokio::sync::RwLock<Banner>,
    leaderboard: tokio::sync::Mutex<ShipLeaderboard>,
    cache: tokio::sync::Mutex<SearchCache>,
}

impl DataInner {
    pub async fn new() -> Self {
        DataInner {
            client: reqwest::Client::new(),
            patron: Arc::new(parking_lot::RwLock::new(Patrons::default())),
            expected: Arc::new(parking_lot::RwLock::new(ExpectedJs::load_json().await)),
            ships: Arc::new(parking_lot::RwLock::new(ShipsPara::load_json().await)),
            constant: parking_lot::RwLock::new(LittleConstant::load_json().await),
            link: tokio::sync::RwLock::new(Linked::load_json().await),
            wg_api_token: env::var("WG_API").expect("Missing WG_API TOKEN"),
            guild_default: tokio::sync::RwLock::new(GuildDefaultRegion::load_json().await),
            banner: tokio::sync::RwLock::new(Banner::load_json().await),
            leaderboard: tokio::sync::Mutex::new(ShipLeaderboard::load_json().await),
            cache: tokio::sync::Mutex::new(SearchCache::new()),
        }
    }
}
