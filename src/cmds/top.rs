use std::time::{SystemTime, UNIX_EPOCH};

use poise::{
    CreateReply,
    serenity_prelude::{CreateActionRow, CreateAttachment, CreateButton},
};

use crate::{
    Context, Data, Error,
    dc_utils::{Args, ContextAddon, UserAddon, autocomplete},
    structs::{Region, Ship, ShipLeaderboardPlayer, ShipLeaderboardShip, StatisticValueType},
    template_data::{KLeaderboardTemplate, LeaderboardTemplate, Render},
    utils::{IsacError, IsacInfo, wws_api::WowsApi},
};

pub fn top_hybrid() -> poise::Command<Data, Error> {
    poise::Command {
        prefix_action: top_prefix().prefix_action,
        slash_action: top().slash_action,
        aliases: top_prefix().aliases,
        ..top()
    }
}

#[poise::command(prefix_command, aliases("dalao"))]
pub async fn top_prefix(ctx: Context<'_>, #[rest] mut args: Args) -> Result<(), Error> {
    let region = args.parse_region(&ctx).await?;
    let ship = args.parse_ship(&ctx).await?;
    func_top(ctx, region, ship).await
}

/// The top players on the specific ship's leaderboard (sorted by PR)
#[poise::command(slash_command)]
pub async fn top(
    ctx: Context<'_>,
    #[description = "warship's name"]
    #[rename = "warship"]
    #[autocomplete = "autocomplete::ship"]
    ship_name: String,
    #[description = "specific region, default: depend on server's default"] region: Option<Region>,
) -> Result<(), Error> {
    let ship = ctx.data().ships.read().search_name(&ship_name, 1)?.first();
    let region = region.unwrap_or_default();
    func_top(ctx, region, ship).await
}

pub fn btop_hybrid() -> poise::Command<Data, Error> {
    poise::Command {
        prefix_action: btop_prefix().prefix_action,
        slash_action: btop().slash_action,
        aliases: btop_prefix().aliases,
        ..btop()
    }
}

#[poise::command(prefix_command, aliases("ktop"))]
pub async fn btop_prefix(ctx: Context<'_>, #[rest] mut args: Args) -> Result<(), Error> {
    let region = args.parse_region(&ctx).await?;
    let ship = args.parse_ship(&ctx).await?;
    func_btop(ctx, region, ship).await
}

/// The top players on the specific ship's Kokomi leaderboard (sorted by base exp)
#[poise::command(slash_command)]
pub async fn btop(
    ctx: Context<'_>,
    #[description = "warship's name"]
    #[rename = "warship"]
    #[autocomplete = "autocomplete::ship"]
    ship_name: String,
    #[description = "specific region, default: depend on server's default"] region: Option<Region>,
) -> Result<(), Error> {
    let ship = ctx.data().ships.read().search_name(&ship_name, 1)?.first();
    let region = region.unwrap_or_default();
    func_btop(ctx, region, ship).await
}

async fn fetch_and_cache_kokomi(
    ctx: &Context<'_>,
    region: &Region,
    ship: &Ship,
) -> Result<Vec<ShipLeaderboardPlayer>, IsacError> {
    let kokomi_token = ctx
        .data()
        .kokomi_api_token
        .as_deref()
        .ok_or(IsacInfo::GeneralError {
            msg: "Kokomi API token not set in env".to_string(),
        })?;

    let res_json = WowsApi::new(ctx)
        .reqwest(
            region.kokomi_url(format!(
                "/api/external/ship/ranking/{ship_id}/",
                ship_id = ship.ship_id
            )),
            |b| {
                b.header("Access-Token", kokomi_token)
                    .query(&[("size", "100"), ("dogtag", "0")])
            },
        )
        .await?
        .json::<serde_json::Value>()
        .await?;

    if res_json.get("status") != Some(&serde_json::Value::String("ok".to_string())) {
        Err(IsacInfo::GeneralError {
            msg: format!(
                "Kokomi API error: {}",
                res_json
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error")
            ),
        })?;
    }

    let players_val = res_json["data"].as_array().ok_or(IsacInfo::GeneralError {
        msg: format!("No one on the leaderboard of `{}` yet", ship.name),
    })?;

    let mut players = Vec::new();
    for row in players_val {
        let parse_clan = |input: &serde_json::Value| -> String {
            match input.as_str() {
                Some(tag) => format!("[{tag}]"),
                None => "".to_string(),
            }
        };

        let rating = row["rating"].as_f64().unwrap_or(0.0);
        let win_rate = row["win_rate"].as_f64().unwrap_or(0.0);
        let avg_frags = row["avg_frags"].as_f64().unwrap_or(0.0);
        let avg_damage = row["avg_damage"].as_f64().unwrap_or(0.0);
        let avg_exp = row["avg_exp"].as_f64().unwrap_or(0.0);

        let player = ShipLeaderboardPlayer {
            color: "".to_string(),
            rank: 0,
            clan: parse_clan(&row["clan_tag"]),
            ign: row["username"].as_str().unwrap_or("Unknown").to_string(),
            uid: row["account_id"].as_u64().unwrap_or(0),
            battles: row["battles"].as_u64().unwrap_or(0),
            pr: StatisticValueType::Pr {
                value: Some(rating),
            }
            .into(),
            winrate: StatisticValueType::Winrate { value: win_rate }.into(),
            frags: StatisticValueType::Frags { value: avg_frags }.into(),
            dmg: StatisticValueType::ShipDmg {
                expected_js: &ctx.data().expected,
                value: avg_damage,
                ship_id: &ship.ship_id,
            }
            .into(),
            exp: StatisticValueType::Exp { value: avg_exp }.into(),
        };
        players.push(player);
    }

    // Sort by PR
    players.sort_by(|a, b| {
        b.pr.value
            .partial_cmp(&a.pr.value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (i, p) in players.iter_mut().enumerate() {
        p.rank = (i + 1) as u64;
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    ctx.data().kleaderboard.lock().await.insert(
        region,
        ship.ship_id,
        ShipLeaderboardShip {
            players: players.clone(),
            last_updated_at: timestamp,
        },
    );

    Ok(players)
}

async fn func_top(ctx: Context<'_>, region: Region, ship: Ship) -> Result<(), Error> {
    let _typing = ctx.typing().await;
    let lb_players = ctx
        .data()
        .kleaderboard
        .lock()
        .await
        .get_ship(&region, &ship.ship_id, true);

    let mut lb_players = match lb_players {
        Some(p) => p,
        None => match fetch_and_cache_kokomi(&ctx, &region, &ship).await {
            Ok(pr_players) => pr_players,
            Err(err) => {
                let _msg = ctx
                    .send(
                        CreateReply::default()
                            .content(err.to_string())
                            .components(vec![CreateActionRow::Buttons(vec![
                                CreateButton::new_link(
                                    region
                                        .number_url(format!("/ship/{},/", ship.ship_id))
                                        .to_string(),
                                )
                                .label("Stats & Numbers"),
                            ])])
                            .reply(true),
                    )
                    .await?;
                return Ok(());
            }
        },
    };

    let api = WowsApi::new(&ctx);
    let author_rank = if let Ok(author_p) = ctx.author().get_full_player(&ctx).await {
        if let Some((p_index, p)) = lb_players
            .iter()
            .enumerate()
            .find(|(_, p)| p.uid == author_p.uid)
        {
            Some((p_index, p))
        } else if author_p.region != region {
            None
        } else if let Some(stats) =
            author_p
                .single_ship(&api, &ship)
                .await?
                .and_then(|author_ship| {
                    author_ship.to_statistic(
                        &ship.ship_id,
                        ctx.data().expected.as_ref(),
                        crate::structs::Mode::Pvp,
                    )
                })
        {
            let author_ship = ShipLeaderboardPlayer {
                color: "#fff".to_string(),
                rank: 0,
                clan: author_p
                    .clan(&api)
                    .await
                    .map(|c| c.tag.with_brackets())
                    .unwrap_or_default(),
                ign: author_p.ign.clone(),
                uid: author_p.uid,
                battles: stats.battles,
                pr: stats.pr,
                winrate: stats.winrate,
                frags: stats.frags,
                dmg: stats.dmg,
                exp: stats.exp,
            };

            let is_in_top = if let Some(last_player) = lb_players.last() {
                author_ship.pr.value > last_player.pr.value
            } else {
                true
            };

            if is_in_top {
                lb_players.push(author_ship);
                lb_players.sort_by(|a, b| {
                    b.pr.value
                        .partial_cmp(&a.pr.value)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                lb_players
                    .iter_mut()
                    .enumerate()
                    .for_each(|(index, p)| p.rank = (index + 1) as u64);
                lb_players
                    .iter()
                    .enumerate()
                    .find(|(_, p)| p.uid == author_p.uid)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let truncate_len = {
        let mut default_truncate_len: usize = 15;
        if let Some((author_index, _)) = author_rank {
            lb_players[author_index].color = "#ffcc66".to_string();
            if author_index >= default_truncate_len {
                lb_players[default_truncate_len] = lb_players.remove(author_index);
                default_truncate_len += 1;
            };
        }
        default_truncate_len
    };

    lb_players.truncate(truncate_len);

    {
        let patrons_rg = ctx.data().patron.read();
        lb_players.iter_mut().for_each(|p| {
            if patrons_rg.check_player(&p.uid) {
                p.color = "#e85a6b".to_string();
            }
        })
    };

    let ship_id = ship.ship_id;
    let data = LeaderboardTemplate {
        ship,
        region,
        players: lb_players,
    };
    let img = data.render(&ctx.data().client).await?;
    let _msg = ctx
        .send(
            CreateReply::default()
                .attachment(CreateAttachment::bytes(img, "image.png"))
                .components(vec![CreateActionRow::Buttons(vec![
                    CreateButton::new_link(
                        region.number_url(format!("/ship/{ship_id},/")).to_string(),
                    )
                    .label("Stats & Numbers"),
                ])])
                .reply(true),
        )
        .await?;

    Ok(())
}

async fn func_btop(ctx: Context<'_>, region: Region, ship: Ship) -> Result<(), Error> {
    let _typing = ctx.typing().await;
    let lb_players = ctx
        .data()
        .kleaderboard
        .lock()
        .await
        .get_ship(&region, &ship.ship_id, true);

    let pr_players = match lb_players {
        Some(p) => p,
        None => match fetch_and_cache_kokomi(&ctx, &region, &ship).await {
            Ok(players) => players,
            Err(err) => {
                let _msg = ctx
                    .send(
                        CreateReply::default()
                            .content(err.to_string())
                            .components(vec![CreateActionRow::Buttons(vec![
                                CreateButton::new_link(
                                    region
                                        .number_url(format!("/ship/{},/", ship.ship_id))
                                        .to_string(),
                                )
                                .label("Stats & Numbers"),
                            ])])
                            .reply(true),
                    )
                    .await?;
                return Ok(());
            }
        },
    };

    // Sort by Exp manually for btop
    let mut lb_players = pr_players;
    lb_players.sort_by(|a, b| {
        b.exp
            .value
            .partial_cmp(&a.exp.value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (i, p) in lb_players.iter_mut().enumerate() {
        p.rank = (i + 1) as u64;
    }

    let api = WowsApi::new(&ctx);
    let author_rank = if let Ok(author_p) = ctx.author().get_full_player(&ctx).await {
        if let Some((p_index, p)) = lb_players
            .iter()
            .enumerate()
            .find(|(_, p)| p.uid == author_p.uid)
        {
            Some((p_index, p))
        } else if author_p.region != region {
            None
        } else if let Some(stats) =
            author_p
                .single_ship(&api, &ship)
                .await?
                .and_then(|author_ship| {
                    author_ship.to_statistic(
                        &ship.ship_id,
                        ctx.data().expected.as_ref(),
                        crate::structs::Mode::Pvp,
                    )
                })
        {
            let author_ship = ShipLeaderboardPlayer {
                color: "#fff".to_string(),
                rank: 0,
                clan: author_p
                    .clan(&api)
                    .await
                    .map(|c| c.tag.with_brackets())
                    .unwrap_or_default(),
                ign: author_p.ign.clone(),
                uid: author_p.uid,
                battles: stats.battles,
                pr: stats.pr,
                winrate: stats.winrate,
                frags: stats.frags,
                dmg: stats.dmg,
                exp: stats.exp,
            };

            let is_in_top = if let Some(last_player) = lb_players.last() {
                author_ship.exp.value > last_player.exp.value
            } else {
                true
            };

            if is_in_top {
                lb_players.push(author_ship);
                lb_players.sort_by(|a, b| {
                    b.exp
                        .value
                        .partial_cmp(&a.exp.value)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                lb_players
                    .iter_mut()
                    .enumerate()
                    .for_each(|(index, p)| p.rank = (index + 1) as u64);
                lb_players
                    .iter()
                    .enumerate()
                    .find(|(_, p)| p.uid == author_p.uid)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let truncate_len = {
        let mut default_truncate_len: usize = 15;
        if let Some((author_index, _)) = author_rank {
            lb_players[author_index].color = "#ffcc66".to_string();
            if author_index >= default_truncate_len {
                lb_players[default_truncate_len] = lb_players.remove(author_index);
                default_truncate_len += 1;
            };
        }
        default_truncate_len
    };

    lb_players.truncate(truncate_len);

    {
        let patrons_rg = ctx.data().patron.read();
        lb_players.iter_mut().for_each(|p| {
            if patrons_rg.check_player(&p.uid) {
                p.color = "#e85a6b".to_string();
            }
        })
    };

    let data = KLeaderboardTemplate(LeaderboardTemplate {
        ship,
        region,
        players: lb_players,
    });
    let img = data.render(&ctx.data().client).await?;
    let _msg = ctx
        .send(
            CreateReply::default()
                .attachment(CreateAttachment::bytes(img, "image.png"))
                .reply(true),
        )
        .await?;

    Ok(())
}
