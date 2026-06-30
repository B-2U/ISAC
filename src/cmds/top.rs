use std::time::{SystemTime, UNIX_EPOCH};

use once_cell::sync::Lazy;
use poise::{
    CreateReply,
    serenity_prelude::{CreateActionRow, CreateAttachment, CreateButton},
};
use regex::Regex;
use scraper::{ElementRef, Html, Selector, node::Element};

use crate::{
    Context, Data, Error,
    dc_utils::{Args, ContextAddon, UserAddon, autocomplete},
    structs::{
        Region, Ship, ShipLeaderboardPlayer, ShipLeaderboardShip, StatisticValue,
        StatisticValueType, color::ColorStats,
    },
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
    func_ktop(ctx, region, ship).await
}

/// The top players on the specific ship's leaderboard
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
    // Redirect to Kokomi leaderboard
    func_ktop(ctx, region, ship).await
    // func_top(ctx, region, ship).await
}

pub fn ktop_hybrid() -> poise::Command<Data, Error> {
    poise::Command {
        prefix_action: ktop_prefix().prefix_action,
        slash_action: ktop().slash_action,
        aliases: ktop_prefix().aliases,
        ..ktop()
    }
}

#[poise::command(prefix_command, aliases("btop"))]
pub async fn ktop_prefix(ctx: Context<'_>, #[rest] mut args: Args) -> Result<(), Error> {
    let region = args.parse_region(&ctx).await?;
    let ship = args.parse_ship(&ctx).await?;
    func_ktop(ctx, region, ship).await
}

/// The top players on the specific ship's Kokomi leaderboard
#[poise::command(slash_command)]
pub async fn ktop(
    ctx: Context<'_>,
    #[description = "warship's name"]
    #[rename = "warship"]
    #[autocomplete = "autocomplete::ship"]
    ship_name: String,
    #[description = "specific region, default: depend on server's default"] region: Option<Region>,
) -> Result<(), Error> {
    let ship = ctx.data().ships.read().search_name(&ship_name, 1)?.first();
    let region = region.unwrap_or_default();
    func_ktop(ctx, region, ship).await
}

#[expect(unused)]
async fn func_top(ctx: Context<'_>, region: Region, ship: Ship) -> Result<(), Error> {
    let _typing = ctx.typing().await;
    let lb_players = ctx
        .data()
        .leaderboard
        .lock()
        .await
        .get_ship(&region, &ship.ship_id, true);

    let mut lb_players = match lb_players {
        Some(p) => p,
        None => {
            match fetch_ship_leaderboard(&ctx, &region, &ship, LeaderboardSource::WowsNumber).await
            {
                Ok(lb_players) => lb_players,
                // parsing failed caused by cloudflare, give user the button to wows number
                Err(err) => {
                    let _msg = ctx
                        .send(
                            CreateReply::default()
                                .content(format!(
                                    "{err}\n`top` is currently unavailable, try `btop` instead"
                                ))
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
            }
        }
    };
    let api = WowsApi::new(&ctx);
    // if author linked and not hidden
    let author_rank = if let Ok(author_p) = ctx.author().get_full_player(&ctx).await {
        // author on the leaderboard, only highlight needed
        if let Some((p_index, p)) = lb_players
            .iter()
            .enumerate()
            .find(|(_, p)| p.uid == author_p.uid)
        {
            Some((p_index, p))
        }
        // if the author is not in the same region as the request, skip it
        else if author_p.region != region {
            None
        }
        // author not on the leaderboard, try to fetch his stats
        else if let Some(stats) =
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
                exp: Default::default(),
            };
            // if author in top 100, push him in, sort and rank
            if author_ship.pr.value > lb_players.last().unwrap().pr.value {
                lb_players.push(author_ship);
                lb_players.sort_by(|a, b| b.pr.value.partial_cmp(&a.pr.value).unwrap());
                lb_players
                    .iter_mut()
                    .enumerate()
                    .for_each(|(index, p)| p.rank = (index + 1) as u64);
                lb_players
                    .iter()
                    .enumerate()
                    .find(|(_, p)| p.uid == author_p.uid)
            } else {
                // pr < 100th
                None
            }
        } else {
            // not stats in the ship
            None
        }
    } else {
        // not linked
        None
    };
    let truncate_len = {
        let mut default_truncate_len: usize = 15;
        if let Some((author_index, _)) = author_rank {
            // color author
            lb_players[author_index].color = "#ffcc66".to_string();
            if author_index >= default_truncate_len {
                // add one more row in the leaderboard for author
                lb_players[default_truncate_len] = lb_players.remove(author_index);
                default_truncate_len += 1;
            };
        }
        default_truncate_len
    };

    lb_players.truncate(truncate_len);

    // color patrons
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

async fn func_ktop(ctx: Context<'_>, region: Region, ship: Ship) -> Result<(), Error> {
    let _typing = ctx.typing().await;
    let lb_players = ctx
        .data()
        .kleaderboard
        .lock()
        .await
        .get_ship(&region, &ship.ship_id, true);

    let mut lb_players = match lb_players {
        Some(p) => p,
        None => {
            match fetch_ship_leaderboard(&ctx, &region, &ship, LeaderboardSource::Kokomi).await {
                Ok(lb_players) => lb_players,
                // parsing failed caused by cloudflare, give user the button to wows number
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
            }
        }
    };
    let api = WowsApi::new(&ctx);
    // if author linked and not hidden
    let author_rank = if let Ok(author_p) = ctx.author().get_full_player(&ctx).await {
        // author on the leaderboard, only highlight needed
        if let Some((p_index, p)) = lb_players
            .iter()
            .enumerate()
            .find(|(_, p)| p.uid == author_p.uid)
        {
            Some((p_index, p))
        }
        // if the author is not in the same region as the request, skip it
        else if author_p.region != region {
            None
        }
        // author not on the leaderboard, try to fetch his stats
        else if let Some(stats) =
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
            // if author in top 100, push him in, sort and rank
            if author_ship.exp.value > lb_players.last().unwrap().exp.value {
                lb_players.push(author_ship);
                lb_players.sort_by(|a, b| b.exp.value.partial_cmp(&a.exp.value).unwrap());
                lb_players
                    .iter_mut()
                    .enumerate()
                    .for_each(|(index, p)| p.rank = (index + 1) as u64);
                lb_players
                    .iter()
                    .enumerate()
                    .find(|(_, p)| p.uid == author_p.uid)
            } else {
                // pr < 100th
                None
            }
        } else {
            // not stats in the ship
            None
        }
    } else {
        // not linked
        None
    };
    let truncate_len = {
        let mut default_truncate_len: usize = 15;
        if let Some((author_index, _)) = author_rank {
            // color author
            lb_players[author_index].color = "#ffcc66".to_string();
            if author_index >= default_truncate_len {
                // add one more row in the leaderboard for author
                lb_players[default_truncate_len] = lb_players.remove(author_index);
                default_truncate_len += 1;
            };
        }
        default_truncate_len
    };

    lb_players.truncate(truncate_len);

    // color patrons
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

async fn fetch_ship_leaderboard(
    ctx: &Context<'_>,
    region: &Region,
    ship: &Ship,
    source: LeaderboardSource,
) -> Result<Vec<ShipLeaderboardPlayer>, IsacError> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let lb_players = match source {
        LeaderboardSource::WowsNumber => {
            fetch_ship_leaderboard_wowsnumber(ctx, region, ship).await?
        }
        LeaderboardSource::Kokomi => fetch_ship_leaderboard_kokomi(ctx, region, ship).await?,
    };
    match source {
        LeaderboardSource::WowsNumber => {
            ctx.data().leaderboard.lock().await.insert(
                region,
                ship.ship_id,
                ShipLeaderboardShip {
                    players: lb_players.clone(),
                    last_updated_at: timestamp.as_secs(),
                },
            );
        }
        LeaderboardSource::Kokomi => {
            ctx.data().kleaderboard.lock().await.insert(
                region,
                ship.ship_id,
                ShipLeaderboardShip {
                    players: lb_players.clone(),
                    last_updated_at: timestamp.as_secs(),
                },
            );
        }
    };

    Ok(lb_players)
}

async fn fetch_ship_leaderboard_wowsnumber(
    ctx: &Context<'_>,
    region: &Region,
    ship: &Ship,
) -> Result<Vec<ShipLeaderboardPlayer>, IsacError> {
    let res_text = WowsApi::new(ctx)
        .ureq(
            region.number_url(format!("/ship/{},/", ship.ship_id)),
            |b| b,
        )
        .await?
        .read_to_string()?;
    let html = Html::parse_document(&res_text);
    // Find the ranking table
    let table_selector = Selector::parse(".ranking-table").unwrap();
    let table = match html.select(&table_selector).count() {
        5 => Err(IsacInfo::GeneralError {
            msg: format!("No one on the leaderboard of `{}` yet", ship.name),
        })?,
        6 => html.select(&table_selector).nth(5).expect("no way to fail"),
        _ => Err(IsacError::Info(IsacInfo::GeneralError {
            msg: "Parsing failed".to_string(),
        }))?,
    };

    // Parse cells in the table
    let row_selector = Selector::parse("tbody>tr").unwrap();
    let a_selector = Selector::parse("a").unwrap();
    let td_selector = Selector::parse("td").unwrap();
    let span_selector = Selector::parse("span").unwrap();

    static IGN_UID_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"/player/(\d+),([^/]+)/").unwrap());
    static COLOR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"#[a-zA-Z\d]{6}").unwrap());

    let mut leader_board = vec![];

    let get_color_value = |element: ElementRef<'_>| -> StatisticValue {
        let span = element.select(&span_selector).next().unwrap();
        let color: ColorStats = COLOR_RE
            .captures(span.value().attr("style").unwrap())
            .unwrap()
            .get(0)
            .unwrap()
            .as_str()
            .parse()
            .unwrap();
        let winrate = span
            .text()
            .next()
            .unwrap()
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '%')
            .collect::<String>()
            .parse::<f64>()
            .unwrap_or(0.0);
        StatisticValue {
            value: winrate,
            color,
        }
    };
    let get_uid_and_ign = |value: &Element| -> (u64, String) {
        if let Some(captures) = IGN_UID_RE.captures(value.attr("href").unwrap()) {
            let uid = captures
                .get(1)
                .unwrap()
                .as_str()
                .parse::<u64>()
                .unwrap_or(00000);
            let ign = captures.get(2).unwrap().as_str().to_string();
            (uid, ign)
        } else {
            (00000, "---".to_string())
        }
    };
    for row in table.select(&row_selector) {
        // Parse values in each row
        let values: Vec<_> = row.select(&td_selector).collect();

        let rank = values[0]
            .text()
            .next()
            .unwrap()
            .parse::<u64>()
            .unwrap_or_default();
        let value_1 = values[1].select(&a_selector).collect::<Vec<_>>();
        let (clan, (uid, ign)) = if value_1.len() == 2 {
            let clan = value_1[0].text().collect::<String>();
            let value = value_1[1].value();
            (clan, get_uid_and_ign(value))
        } else {
            ("".to_string(), get_uid_and_ign(value_1[0].value()))
        };
        let battles = values[2]
            .select(&span_selector)
            .next()
            .unwrap()
            .text()
            .next()
            .unwrap()
            .replace(' ', "")
            .parse::<u64>()
            .unwrap();

        let player = ShipLeaderboardPlayer {
            color: "".to_string(),
            rank,
            clan,
            ign,
            uid,
            battles,
            pr: get_color_value(values[3]),
            winrate: get_color_value(values[4]),
            frags: get_color_value(values[5]),
            dmg: get_color_value(values[7]),
            exp: Default::default(),
        };
        leader_board.push(player);
        // player.insert("rank".to_string(), values[0].text().collect::<String>());

        // Continue parsing other values similarly...
        // You can adapt the code to handle more complex parsing

        // Print the parsed player data
    }
    Ok(leader_board)
}

/// sort by bxp
async fn fetch_ship_leaderboard_kokomi(
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
        })?
    }

    let players = {
        let Some(players) = res_json["data"].as_array() else {
            Err(IsacInfo::GeneralError {
                msg: format!("No one on the leaderboard of `{}` yet", ship.name),
            })?
        };
        // "rank": 1,
        // "account_id": 2019176341,
        // "username": "Honoka_Sukidayo",
        // "clan_id": 2000038692,
        // "clan_tag": "FOCAL",
        // "clan_league": 4,
        // "battles": 82,
        // "rating": 3572,
        // "win_rate": 80.49,
        // "win_rate_level": 8,
        // "avg_damage": 144484,
        // "avg_damage_level": 8,
        // "avg_frags": 1.67,
        // "avg_frags_level": 8,
        // "avg_exp": 1704,
        players.clone()
    };

    let mut leader_board = vec![];
    for (index, row) in players.into_iter().enumerate() {
        fn parse_clan(input: &serde_json::Value) -> String {
            match input.as_str() {
                Some(tag) => format!("[{tag}]"),
                None => "".to_string(), // null when no clan
            }
        }
        let player = ShipLeaderboardPlayer {
            color: "".to_string(),
            rank: index as u64 + 1,
            clan: parse_clan(&row["clan_tag"]),
            ign: row["username"].as_str().unwrap().to_string(),
            uid: row["account_id"].as_u64().unwrap(),
            battles: row["battles"].as_u64().unwrap(),
            pr: StatisticValue {
                value: row["rating"].as_f64().unwrap(),
                color: ColorStats::parse_kokomi_class(row["win_rate_level"].as_u64().unwrap()),
            },
            winrate: StatisticValue {
                value: row["win_rate"].as_f64().unwrap(),
                color: ColorStats::parse_kokomi_class(row["win_rate_level"].as_u64().unwrap()),
            },
            frags: StatisticValue {
                value: row["avg_frags"].as_f64().unwrap(),
                color: ColorStats::parse_kokomi_class(row["avg_frags_level"].as_u64().unwrap()),
            },
            dmg: StatisticValue {
                value: row["avg_damage"].as_f64().unwrap(),
                color: ColorStats::parse_kokomi_class(row["avg_damage_level"].as_u64().unwrap()),
            },
            exp: StatisticValueType::Exp {
                value: row["avg_exp"].as_f64().unwrap(),
            }
            .into(),
        };
        leader_board.push(player);
    }
    Ok(leader_board)
}

enum LeaderboardSource {
    WowsNumber,
    Kokomi,
}
