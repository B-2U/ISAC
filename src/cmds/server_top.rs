use futures::{future::join_all, StreamExt};
use itertools::Itertools;
use poise::{serenity_prelude::CreateAttachment, CreateReply};

use crate::{
    dc_utils::{autocomplete, Args, ContextAddon, UserAddon},
    structs::{PlayerSnapshots, Ship},
    template_data::{Render, ServerTopPlayer, ServerTopTemplate},
    utils::{wws_api::WowsApi, IsacError, IsacInfo},
    Context, Data, Error,
};

pub fn server_top_hybrid() -> poise::Command<Data, Error> {
    poise::Command {
        prefix_action: server_top_prefix().prefix_action,
        slash_action: server_top().slash_action,
        aliases: server_top_prefix().aliases,
        ..server_top()
    }
}

#[poise::command(prefix_command, aliases("stop"), user_cooldown = 7)]
pub async fn server_top_prefix(ctx: Context<'_>, #[rest] mut args: Args) -> Result<(), Error> {
    let ship = args.parse_ship(&ctx).await?;
    func_server_top(ctx, ship).await
}

/// Top 15 players in the ship in the discord server (min battles = 10)
#[poise::command(slash_command)]
pub async fn server_top(
    ctx: Context<'_>,
    #[description = "warship's name"]
    #[rename = "warship"]
    #[autocomplete = "autocomplete::ship"]
    ship_name: String,
) -> Result<(), Error> {
    let ship = ctx.data().ships.read().search_name(&ship_name, 1)?.first();
    func_server_top(ctx, ship).await
}

async fn func_server_top(ctx: Context<'_>, ship: Ship) -> Result<(), Error> {
    let (guild_name, guild_id) = {
        let guild = ctx.guild().ok_or(IsacError::Info(IsacInfo::GeneralError {
            msg: "Not a server".to_string(),
        }))?;
        (guild.name.clone(), guild.id)
    };

    let _typing = ctx.typing().await;

    let time = std::time::Instant::now();

    // linked users
    let p_players = join_all(
        guild_id
            .members(ctx, None, None)
            .await?
            .into_iter()
            .map(|m| async move { m.user.get_player(&ctx).await }),
    )
    .await
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    let p_players_len = p_players.len(); // just for debugging

    let api = WowsApi::new(&ctx);
    let api_ref = &api;
    let expected = ctx.data().expected.as_ref();
    // the players who have stats in that ship
    // let mut players = HashMap::new();

    let stream = futures::stream::iter(p_players.into_iter().map(|p_player| async move {
        let record = match PlayerSnapshots::load(p_player).await {
            Some(snapshots) => Some(snapshots.latest_snapshot().expect("shouldn't be None")),
            // no snapshot, fetch and save
            None => {
                if let Ok(current_ships) = p_player.all_ships(api_ref).await {
                    let mut snapshots = PlayerSnapshots::init(p_player).await;
                    snapshots.insert(current_ships.clone());
                    snapshots.save().await;
                    Some(current_ships)
                } else {
                    // maybe hidden profile or sth
                    None
                }
            }
        };
        // None if the player didn't play the ship
        record
            .and_then(|r| {
                r.get_ship(&ship.ship_id).and_then(|s| {
                    s.to_statistic(&ship.ship_id, expected, crate::structs::Mode::Pvp)
                        .filter(|stats| stats.battles >= 10)
                })
            })
            .map(|s| (p_player, s))
    }))
    .buffer_unordered(100);
    // append their index after sorted (index, player, stats)
    let mut lb_players = stream
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .sorted_by(|a, b| b.1.pr.value.total_cmp(&a.1.pr.value))
        // multiple users link to same ign
        .dedup_by(|a, b| a.0.uid == b.0.uid)
        .enumerate()
        .map(|(i, (p, s))| (i + 1, p, s))
        .collect::<Vec<_>>();

    // truncate, if user is in the leaderboard, set color and swap its index if needed
    let author_player = ctx.author().get_player(&ctx).await;
    let truncate_len = if let Some((p_index, _p)) = author_player.and_then(|author_p| {
        lb_players
            .iter_mut()
            .enumerate()
            .find(|(_, p)| p.1.uid == author_p.uid)
    }) {
        if p_index >= 15 {
            lb_players.swap(15, p_index);
            16 // user in top 100
        } else {
            15 // user in top 15
        }
    } else {
        15 // user not in leaderboard
    };
    lb_players.truncate(truncate_len);

    // turn PartialPlayer to Player
    let mut lb_players = join_all(lb_players.into_iter().map(|(i, p, s)| async move {
        // if he's cached but hidden after that
        let p = p.full_player(api_ref).await.unwrap_or_default();
        let clan_tag = p
            .clan(&api)
            .await
            .map(|c| c.tag.with_brackets())
            .unwrap_or_default();
        ServerTopPlayer {
            color: "#fff".to_string(),
            rank: i,
            clan: clan_tag,
            player: p,
            stats: s,
        }
    }))
    .await;

    // color patrons and author
    {
        let patrons_rg = ctx.data().patron.read();
        lb_players.iter_mut().take(15).for_each(|p| {
            if patrons_rg.check_player(&p.player.uid) {
                p.color = "#e85a6b".to_string();
            }
            if let Some(author_p) = author_player {
                if author_p.uid == p.player.uid {
                    p.color = "#ffcc66".to_string();
                }
            }
        })
    };
    let data = ServerTopTemplate {
        ship,
        server: guild_name,
        players: lb_players,
    };
    let img = data.render(&ctx.data().client).await?;
    let _msg = ctx
        .send(
            CreateReply::default()
                .attachment(CreateAttachment::bytes(img, "image.png"))
                .reply(true),
        )
        .await?;
    println!(
        "server members: {}, took time: {:.4}s",
        p_players_len,
        time.elapsed().as_secs_f32()
    );
    Ok(())
}
