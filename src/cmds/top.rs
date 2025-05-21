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
    func_top(ctx, region, ship).await
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
    func_top(ctx, region, ship).await
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
    let res_json = WowsApi::new(ctx)
        .reqwest(
            format!(
                "http://129.226.90.10:8010/api/v1/robot/leaderboard/page/{}/{}/",
                region.kokomi_region(),
                ship.ship_id
            ),
            |b| b.query(&[("language", "english")]),
        )
        .await?
        .json::<serde_json::Value>()
        .await?;
    let mut leader_board = vec![];

    let players = {
        let Some(res_data) = res_json["data"].as_object() else {
            Err(IsacInfo::GeneralError {
                msg: format!("No one on the leaderboard of `{}` yet", ship.name),
            })?
        };
        let mut players = res_data["leaderboard"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        players.sort_unstable_by_key(|v| {
            std::cmp::Reverse(v["avg_exp"].as_str().unwrap().parse::<u64>().unwrap())
        });
        players
    };
    for (index, row) in players.into_iter().enumerate() {
        let row = row.as_object().unwrap();
        // "rank": "1",
        // "region": "Asia",
        // "region_id": "1",
        // "clan_tag": "RINA",
        // "clan_id": "2000038851",
        // "user_name": "Blanquillo",
        // "account_id": "2024413063",
        // "battles_count": "2961",
        // "battle_type": "88.65%",
        // "rating": "3365",
        // "rating_info": "3394 - 29",
        // "win_rate": "68.93%",
        // "avg_dmg": "145366",
        // "avg_frags": "1.41",
        // "avg_exp": "1783",
        // "max_dmg": "361274",
        // "max_frags": "8",
        // "max_exp": "3948",
        // "clan_tag_class": 1,
        // "battle_type_class": 8,
        // "rating_class": 8,
        // "win_rate_class": 7,
        // "avg_dmg_class": 8,
        // "avg_frags_class": 7
        fn parse_clan(input: &serde_json::Value) -> String {
            let clan = input.as_str().unwrap();
            if clan == "nan" {
                "".to_string()
            } else {
                format!("[{clan}]")
            }
        }
        let player = ShipLeaderboardPlayer {
            color: "".to_string(),
            // we sort it by ourself, so the rank is not the same as the original
            rank: index as u64 + 1,
            clan: parse_clan(&row["clan_tag"]), // NOTICE: Ahh backward compatibility
            ign: row["user_name"].as_str().unwrap().to_string(),
            uid: row["account_id"].as_str().unwrap().parse().unwrap(),
            battles: row["battles_count"].as_str().unwrap().parse().unwrap(),
            pr: StatisticValue {
                value: row["rating"].as_str().unwrap().parse().unwrap(),
                color: ColorStats::parse_kokomi_class(row["rating_class"].as_u64().unwrap()),
            },
            winrate: StatisticValue {
                value: row["win_rate"]
                    .as_str()
                    .unwrap()
                    .strip_suffix("%")
                    .unwrap()
                    .parse()
                    .unwrap(),
                color: ColorStats::parse_kokomi_class(row["win_rate_class"].as_u64().unwrap()),
            },
            frags: StatisticValue {
                value: row["avg_frags"].as_str().unwrap().parse().unwrap(),
                color: ColorStats::parse_kokomi_class(row["avg_frags_class"].as_u64().unwrap()),
            },
            dmg: StatisticValue {
                value: row["avg_dmg"].as_str().unwrap().parse().unwrap(),
                color: ColorStats::parse_kokomi_class(row["avg_dmg_class"].as_u64().unwrap()),
            },
            exp: StatisticValueType::Exp {
                value: row["avg_exp"].as_str().unwrap().parse().unwrap(),
            }
            .into(),
        };
        leader_board.push(player);
        // player.insert("rank".to_string(), values[0].text().collect::<String>());

        // Continue parsing other values similarly...
        // You can adapt the code to handle more complex parsing

        // Print the parsed player data
    }
    Ok(leader_board)
}

enum LeaderboardSource {
    WowsNumber,
    Kokomi,
}
