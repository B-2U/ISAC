use std::{borrow::Cow, collections::HashMap, time::Duration};

use futures::StreamExt;
use itertools::Itertools;
use poise::serenity_prelude::{
    AttachmentType, ButtonStyle, CreateActionRow, CreateButton, CreateComponents, User,
};

use crate::{
    dc_utils::{auto_complete, Args, ContextAddon, CreateReplyAddon, UserAddon},
    structs::{Mode, PartialPlayer, Ship, ShipClass, ShipTier, Statistic, StatisticValueType},
    template_data::{
        OverallCwTemplate, OverallCwTemplateSeason, OverallTemplate, OverallTemplateClass,
        OverallTemplateDiv, OverallTemplateTier, Render, SingleShipTemplate,
    },
    utils::{wws_api::WowsApi, IsacError, IsacInfo},
    Context, Data, Error,
};

pub fn wws_hybrid() -> poise::Command<Data, Error> {
    poise::Command {
        prefix_action: wws_prefix().prefix_action,
        slash_action: wws().slash_action,
        ..wws()
    }
}

/// Account overall / Specific warship's stats
#[poise::command(slash_command, user_cooldown = 5)]
pub async fn wws(
    ctx: Context<'_>,
    #[description = "specific warship, default: account's overall stats"]
    #[rename = "warship"]
    #[autocomplete = "auto_complete::ship"]
    ship_name: Option<String>,
    #[description = "player's ign, default: yourself"]
    #[autocomplete = "auto_complete::player"]
    player: Option<String>, // the String is a Serialized PartialPlayer struct
    #[description = "@ping / discord user's ID, default: yourself"]
    #[rename = "user"]
    discord_user: Option<String>,
    #[description = "battle type, default: pvp"] battle_type: Option<Mode>,
) -> Result<(), Error> {
    let partial_player = if let Some(player) =
        player.and_then(|player_str| serde_json::from_str::<PartialPlayer>(&player_str).ok())
    {
        player
    } else {
        let user = if let Some(discord_user_str) = discord_user {
            User::convert_strict(
                ctx.serenity_context(),
                ctx.guild_id(),
                None,
                &discord_user_str,
            )
            .await
            .unwrap_or(ctx.author().clone())
        } else {
            ctx.author().clone()
        };
        ctx.data()
            .link_js
            .read()
            .await
            .get(&user.id)
            .ok_or(IsacError::Info(IsacInfo::UserNotLinked {
                user_name: Some(user.name.clone()),
            }))?
    };

    if let Some(ship_name) = ship_name {
        // wws ship
        let ship = ctx
            .data()
            .ship_js
            .read()
            .search_name(&ship_name, 1)?
            .first();
        let battle_type = battle_type.unwrap_or_default();
        func_ship(&ctx, partial_player, ship, battle_type).await?;
    } else {
        // wws
        func_wws(&ctx, partial_player).await?;
    }
    Ok(())
}

#[poise::command(prefix_command)]
pub async fn wws_prefix(ctx: Context<'_>, #[rest] mut args: Args) -> Result<(), Error> {
    let typing = ctx.typing().await;

    let partial_player = args.parse_user(&ctx).await?;
    typing.stop();
    if args.is_empty() {
        // wws
        func_wws(&ctx, partial_player).await?;
    } else {
        // wws ship
        let mode = args.parse_mode().unwrap_or_default();
        let ship = args.parse_ship(&ctx).await?;
        func_ship(&ctx, partial_player, ship, mode).await?;
    }

    Ok(())
}

async fn func_ship(
    ctx: &Context<'_>,
    partial_player: PartialPlayer,
    ship: Ship,
    mode: Mode,
) -> Result<(), Error> {
    let _typing = ctx.typing().await;
    let api = WowsApi::new(ctx);
    let player = partial_player.full_player(&api).await?;
    let clan = player.clan(&api).await;
    let ship_stats = player.single_ship(&api, &ship).await?.unwrap_or_default(); // let it default, we will raise error belowed

    // getting player rank in the leaderboard
    let ranking = ctx
        .data()
        .leaderboard
        .lock()
        .await
        .get_ship(&player.region, &ship.ship_id, false)
        .and_then(|players| {
            players
                .into_iter()
                .find(|p| p.uid == player.uid)
                .map(|p| p.rank)
        });

    let data = SingleShipTemplate::new(
        ctx,
        ship,
        ranking,
        mode.render_name().to_string(),
        ship_stats,
        mode,
        clan,
        player,
    )?;
    let img = data.render(&ctx.data().client).await?;
    let _msg = ctx
        .send(|b| {
            b.attachment(AttachmentType::Bytes {
                data: Cow::Borrowed(&img),
                filename: "image.png".to_string(),
            })
            .reply(true)
        })
        .await?
        .into_message()
        .await?;
    Ok(())
}

pub async fn func_wws(ctx: &Context<'_>, partial_player: PartialPlayer) -> Result<(), Error> {
    let typing = ctx.typing().await;
    let api = WowsApi::new(ctx);
    let player = partial_player.full_player(&api).await?;
    let clan = player.clan(&api).await;

    // wws
    let ships = player.all_ships(&api).await?;
    let div = OverallTemplateDiv::new(
        ships
            .to_statistic(&ctx.data().expected_js, Mode::Pvp)
            .unwrap_or_default(),
        ships
            .to_statistic(&ctx.data().expected_js, Mode::Solo)
            .unwrap_or_default(),
        ships
            .to_statistic(&ctx.data().expected_js, Mode::Div2)
            .unwrap_or_default(),
        ships
            .to_statistic(&ctx.data().expected_js, Mode::Div3)
            .unwrap_or_default(),
    );
    let class: OverallTemplateClass = ships
        .clone()
        .sort_class(ctx)
        .await
        .into_iter()
        .map(|(class, ships)| {
            (
                class,
                ships
                    .to_statistic(&ctx.data().expected_js, Mode::Pvp)
                    .unwrap_or_default(),
            )
        })
        .collect::<HashMap<ShipClass, Statistic>>()
        .into();
    let tier: OverallTemplateTier = ships
        .sort_tier(ctx)
        .await
        .into_iter()
        .map(|(class, ships)| {
            (
                class,
                ships
                    .to_statistic(&ctx.data().expected_js, Mode::Pvp)
                    .unwrap_or_default(),
            )
        })
        .collect::<HashMap<ShipTier, Statistic>>()
        .into();
    let overall_data = OverallTemplate {
        div,
        tier,
        class,
        clan: clan.clone(),
        user: player.clone(),
    };
    let img = overall_data.render(&ctx.data().client).await?;

    let mut view = WwsView::new(overall_data, partial_player);
    let mut msg = ctx
        .send(|b| {
            b.attachment(AttachmentType::Bytes {
                data: Cow::Borrowed(&img),
                filename: "image.png".to_string(),
            })
            .set_components(view.build())
            .reply(true)
        })
        .await?
        .into_message()
        .await?;
    typing.stop();

    // waiting for interactions
    while let Some(interaction) = msg
        .await_component_interactions(ctx)
        .timeout(Duration::from_secs(60))
        .author_id(ctx.author().id)
        .build()
        .next()
        .await
    {
        let _typing = ctx.typing().await;
        let custom_id = interaction.data.custom_id.as_str();

        if custom_id == "overall_tier" {
            // disable button first
            view.by_tier_btn.disabled(true);
            let _ok = interaction
                .edit_original_message(ctx, |m| {
                    m.interaction_response_data(|d| d.set_components(view.build()))
                })
                .await;
            // generate then send image
            let img_2 = view.overall_data.render_tiers(&ctx.data().client).await?;
            let _ok = msg
                .edit(ctx, |m| {
                    m.attachment(AttachmentType::Bytes {
                        data: Cow::Borrowed(&img_2),
                        filename: "image.png".to_string(),
                    })
                })
                .await;
        } else if custom_id == "overall_cw" {
            // disable button first
            view.cw_btn.disabled(true);
            let _ok = interaction
                .edit_original_message(ctx, |m| {
                    m.interaction_response_data(|d| d.set_components(view.build()))
                })
                .await;
            let api = WowsApi::new(ctx);
            let data_2 = view.player.clan_battle_season_stats(&api).await?;
            let overall_cw_data = OverallCwTemplate {
                seasons: data_2
                    .seasons
                    .into_iter()
                    .filter(|s| s.season_id <= 200) // remove some other weird seasons
                    .sorted_by(|a, b| b.season_id.cmp(&a.season_id))
                    .map(|s| OverallCwTemplateSeason {
                        season_id: s.season_id,
                        winrate: StatisticValueType::Winrate {
                            value: s.wins as f64 / s.battles as f64 * 100.0,
                        }
                        .into(),
                        battles: s.battles,
                        dmg: StatisticValueType::OverallDmg {
                            value: s.damage_dealt as f64 / s.battles as f64,
                        }
                        .into(),
                        frags: StatisticValueType::Frags {
                            value: s.frags as f64 / s.battles as f64,
                        }
                        .into(),
                        potential: s.art_agro / s.battles,
                        scout: s.damage_scouting / s.battles,
                    })
                    .collect::<Vec<_>>(),
                clan: clan.clone(),
                user: player.clone(),
            };
            let img_2 = overall_cw_data.render(&ctx.data().client).await?;
            let _ok = msg
                .edit(ctx, |m| {
                    m.attachment(AttachmentType::Bytes {
                        data: Cow::Borrowed(&img_2),
                        filename: "image.png".to_string(),
                    })
                })
                .await;
        }
    }
    // timeout;
    msg.edit(ctx, |m| m.set_components(view.timeout().build()))
        .await?;
    Ok(())
}

struct WwsView {
    pub overall_data: OverallTemplate,
    pub player: PartialPlayer,
    by_tier_btn: CreateButton,
    cw_btn: CreateButton,
}

impl WwsView {
    fn new(overall_data: OverallTemplate, player: PartialPlayer) -> Self {
        let by_tier_btn = CreateButton::default()
            .custom_id("overall_tier")
            .style(poise::serenity_prelude::ButtonStyle::Secondary)
            .label("stats by tier")
            .to_owned();
        let cw_btn = CreateButton::default()
            .custom_id("overall_cw")
            .style(poise::serenity_prelude::ButtonStyle::Secondary)
            .label("CB seasons")
            .to_owned();
        Self {
            overall_data,
            player,
            by_tier_btn,
            cw_btn,
        }
    }

    fn build(&self) -> CreateComponents {
        let mut view = CreateComponents::default();
        let mut row = CreateActionRow::default();
        row.add_button(self.by_tier_btn.clone())
            .add_button(self.cw_btn.clone())
            .create_button(|b| {
                b.label("Official")
                    .url(self.player.profile_url().unwrap())
                    .style(ButtonStyle::Link)
            })
            .create_button(|b| {
                b.label("Stats & Numbers")
                    .url(self.player.wows_number_url().unwrap())
                    .style(ButtonStyle::Link)
            });
        view.set_action_row(row);
        view
    }

    fn timeout(&mut self) -> &Self {
        self.by_tier_btn.disabled(true);
        self.cw_btn.disabled(true);
        self
    }
}
