use std::time::{SystemTime, UNIX_EPOCH};

use once_cell::sync::Lazy;
use poise::{
    serenity_prelude::{CreateActionRow, CreateAttachment, CreateButton},
    CreateReply,
};
use regex::Regex;
use scraper::{node::Element, ElementRef, Html, Selector};

use crate::{
    dc_utils::{auto_complete, Args, ContextAddon, UserAddon},
    structs::{
        color::ColorStats, Region, Ship, ShipLeaderboardPlayer, ShipLeaderboardShip, StatisticValue,
    },
    template_data::{LeaderboardTemplate, Render},
    utils::{wws_api::WowsApi, IsacError, IsacInfo},
    Context, Data, Error,
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
    #[autocomplete = "auto_complete::ship"]
    ship_name: String,
    #[description = "specific region, default: depend on server's default"] region: Option<Region>,
) -> Result<(), Error> {
    let ship = ctx
        .data()
        .ship_js
        .read()
        .search_name(&ship_name, 1)?
        .first();
    let region = region.unwrap_or_default();
    func_top(ctx, region, ship).await
}

async fn func_top(ctx: Context<'_>, region: Region, ship: Ship) -> Result<(), Error> {
    let _typing = ctx.typing().await;
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let lb_players = ctx
        .data()
        .leaderboard
        .lock()
        .await
        .get_ship(&region, &ship.ship_id, true);

    let mut lb_players = match lb_players {
        Some(p) => p,
        None => {
            let lb_players = fetch_ship_leaderboard(&ctx, &region, &ship).await?;
            let mut lb_cache = ctx.data().leaderboard.lock().await;

            lb_cache.insert(
                &region,
                ship.ship_id,
                ShipLeaderboardShip {
                    players: lb_players.clone(),
                    last_updated_at: timestamp.as_secs(),
                },
            );
            // TODO: disable saving temperally, need to fix it
            // lb_cache.save_json_sync();
            lb_players
        }
    };
    let api = WowsApi::new(&ctx);
    // if author linked and not hidden
    let author_rank = if let Some(author_p) = match ctx.author().get_player(&ctx).await {
        Some(author_pp) => author_pp.full_player(&api).await.ok(),
        None => None,
    } {
        if let Some((p_index, p)) = lb_players
            .iter()
            .enumerate()
            .find(|(_, p)| p.uid == author_p.uid)
        {
            Some((p_index, p))
        }
        // author not in the leaderboard, try to fetch his stats
        else if let Some(stats) =
            author_p
                .single_ship(&api, &ship)
                .await?
                .and_then(|author_ship| {
                    author_ship.to_statistic(
                        &ship.ship_id,
                        ctx.data().expected_js.as_ref(),
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
                planes: stats.planes,
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
    let truncate_len = match author_rank {
        Some((index, _)) => {
            // color author
            lb_players[index].color = "#ffcc66".to_string();
            match index >= 15 {
                true => {
                    lb_players.swap(15, index);
                    16 // author in top 100
                }
                false => 15, // author in top 15
            }
        }
        None => 15, // author not in leaderboard
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

pub async fn fetch_ship_leaderboard(
    ctx: &Context<'_>,
    region: &Region,
    ship: &Ship,
) -> Result<Vec<ShipLeaderboardPlayer>, IsacError> {
    let res_text = ctx
        .data()
        .client
        .get(region.number_url(format!("/ship/{},/", ship.ship_id)))
        .send()
        .await?
        .text()
        .await?;
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
            planes: get_color_value(values[11]),
        };
        leader_board.push(player);
        // player.insert("rank".to_string(), values[0].text().collect::<String>());

        // Continue parsing other values similarly...
        // You can adapt the code to handle more complex parsing

        // Print the parsed player data
    }
    Ok(leader_board)
}
