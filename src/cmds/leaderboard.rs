use std::{
    borrow::Cow,
    time::{SystemTime, UNIX_EPOCH},
};

use poise::serenity_prelude::AttachmentType;
use regex::Regex;
use scraper::{node::Element, ElementRef, Html, Selector};

use crate::{
    dc_utils::{auto_complete, Args, ContextAddon, UserAddon},
    utils::{
        structs::{
            template_data::{LeaderboardTemplate, Render},
            Region, Ship, ShipId, ShipLeaderboardPlayer, ShipLeaderboardShip, StatisticValue,
        },
        IsacError, IsacInfo, LoadSaveFromJson,
    },
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
    ship_id: u64,
    #[description = "specific region, default: depend on server's default"] region: Option<Region>,
) -> Result<(), Error> {
    let Some(ship) = ShipId(ship_id).get_ship(&ctx.data().ship_js) else {
        Err(IsacInfo::AutoCompleteError)?
    };
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
        .expect("lb poisoned!")
        .get_ship(&region, &ship.ship_id, true);

    let mut lb_players = match lb_players {
        Some(p) => p,
        None => {
            let lb_players = fetch_ship_leaderboard(&ctx, &region, &ship).await?;
            let mut lb_cache = ctx.data().leaderboard.lock().expect("lb poisoned!");

            lb_cache.insert(
                &region,
                ship.ship_id,
                ShipLeaderboardShip {
                    players: lb_players.clone(),
                    last_updated_at: timestamp.as_secs(),
                },
            );
            lb_cache.save_json_sync();
            lb_players
        }
    };

    // if user is in the leaderboard, set color and swap its index if needed
    let truncate_len = if let Some((p_index, p)) =
        ctx.author().get_player(&ctx).await.and_then(|player| {
            lb_players
                .iter_mut()
                .enumerate()
                .find(|(_, p)| p.uid == player.uid)
        }) {
        p.color = "#ffcc66".to_string();
        if p_index >= 15 {
            lb_players.swap(15, p_index);
            16
        } else {
            15
        }
    } else {
        15
    };
    lb_players.truncate(truncate_len);

    let ship_id = ship.ship_id;
    let data = LeaderboardTemplate {
        ship,
        region,
        players: lb_players,
    };
    let img = data.render(&ctx.data().client).await?;
    let _msg = ctx
        .send(|b| {
            b.attachment(AttachmentType::Bytes {
                data: Cow::Borrowed(&img),
                filename: "image.png".to_string(),
            })
            .components(|c| {
                c.create_action_row(|r| {
                    r.create_button(|b| {
                        b.style(poise::serenity_prelude::ButtonStyle::Link)
                            .url(
                                region
                                    .number_url(format!("/ship/{ship_id},/"))
                                    .unwrap()
                                    .to_string(),
                            )
                            .label("Stats & Numbers")
                    })
                })
            })
            .reply(true)
        })
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
        .get(region.number_url(format!("/ship/{},/", ship.ship_id))?)
        .send()
        .await?
        .text()
        .await?;
    let html = Html::parse_document(&res_text);
    // Find the ranking table
    let table_selector = Selector::parse(".ranking-table").unwrap();
    let Some(table) = html.select(&table_selector).skip(5).next() else {
        Err(IsacError::UnknownError("Leaderboard parsing failed".into()))?
    };

    // Parse cells in the table
    let row_selector = Selector::parse("tbody>tr").unwrap();
    let a_selector = Selector::parse("a").unwrap();
    let td_selector = Selector::parse("td").unwrap();
    let span_selector = Selector::parse("span").unwrap();

    let ign_uid_re = Regex::new(r"/player/(\d+),([^/]+)/").unwrap();
    let color_re = Regex::new(r"#[a-zA-Z\d]{6}").unwrap();

    let mut leader_board = vec![];

    let get_color_value = |element: ElementRef<'_>| -> StatisticValue {
        let span = element.select(&span_selector).next().unwrap();
        let color = color_re
            .captures(&span.value().attr("style").unwrap())
            .unwrap()
            .get(0)
            .unwrap()
            .as_str()
            .to_string();
        // let color = if color == "#A00DC5" { "#9d42f3" } else { color }.to_string();
        StatisticValue {
            value: span
                .text()
                .next()
                .unwrap()
                .chars()
                .filter(|c| !c.is_whitespace() && *c != '%')
                .collect(),
            color,
        }
    };
    let get_uid_and_ign = |value: &Element| -> (u64, String) {
        if let Some(captures) = ign_uid_re.captures(value.attr("href").unwrap()) {
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
            (clan, get_uid_and_ign(&value))
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
