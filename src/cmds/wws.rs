use std::{collections::HashMap, time::Duration};

use futures::StreamExt;
use itertools::Itertools;
use poise::{
    CreateReply,
    serenity_prelude::{
        ButtonStyle, CreateActionRow, CreateAttachment, CreateButton, CreateInteractionResponse,
        EditAttachments, EditMessage, User,
    },
};

use crate::{
    Context, Data, Error,
    dc_utils::{Args, ContextAddon, UserAddon, autocomplete},
    structs::{
        AutocompletePlayer, Mode, PartialPlayer, Ship, ShipClass, ShipTier, Statistic,
        StatisticValueType,
    },
    template_data::{
        OverallCwTemplate, OverallCwTemplateSeason, OverallTemplate, OverallTemplateClass,
        OverallTemplateDiv, OverallTemplateTier, Render, SingleShipTemplate,
    },
    utils::wws_api::WowsApi,
};

pub fn wws_hybrid() -> poise::Command<Data, Error> {
    poise::Command {
        prefix_action: wws_prefix().prefix_action,
        slash_action: wws().slash_action,
        ..wws()
    }
}

/// Account overall / Specific warship's stats
#[poise::command(slash_command, user_cooldown = 3)]
pub async fn wws(
    ctx: Context<'_>,
    #[description = "specific warship, default: account's overall stats"]
    #[rename = "warship"]
    #[autocomplete = "autocomplete::ship"]
    ship_name: Option<String>,
    #[description = "player's ign, default: yourself"]
    #[autocomplete = "autocomplete::player"]
    player: Option<AutocompletePlayer>,
    #[description = "@ping / discord user's ID, default: yourself"]
    #[rename = "user"]
    discord_user: Option<String>,
    #[description = "battle type, default: pvp"] battle_type: Option<Mode>,
) -> Result<(), Error> {
    let partial_player = if let Some(autocomplete_player) = player {
        autocomplete_player.save_user_search_history(&ctx).await;
        autocomplete_player
            .fetch_partial_player(&WowsApi::new(&ctx))
            .await?
    } else {
        let user = if let Some(discord_user_str) = discord_user {
            User::convert_strict(
                ctx.serenity_context(),
                ctx.guild_id(),
                None,
                &discord_user_str,
            )
            .await?
        } else {
            ctx.author().clone()
        };
        user.get_player(&ctx).await?
    };

    if let Some(ship_name) = ship_name {
        // wws ship
        let ship = ctx.data().ships.read().search_name(&ship_name, 1)?.first();
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
    let clan = player.clan(&api).await.ok();
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
        .send(
            CreateReply::default()
                .attachment(CreateAttachment::bytes(img, "image.png"))
                .reply(true),
        )
        .await?
        .into_message()
        .await?;
    Ok(())
}

pub async fn func_wws(ctx: &Context<'_>, partial_player: PartialPlayer) -> Result<(), Error> {
    let typing = ctx.typing().await;
    let api = WowsApi::new(ctx);
    let player = partial_player.full_player(&api).await?;
    let clan = player.clan(&api).await.ok();

    // wws
    let ships = player.all_ships(&api).await?;
    let div = OverallTemplateDiv::new(
        ships
            .to_statistic(&ctx.data().expected, Mode::Pvp)
            .unwrap_or_default(),
        ships
            .to_statistic(&ctx.data().expected, Mode::Solo)
            .unwrap_or_default(),
        ships
            .to_statistic(&ctx.data().expected, Mode::Div2)
            .unwrap_or_default(),
        ships
            .to_statistic(&ctx.data().expected, Mode::Div3)
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
                    .to_statistic(&ctx.data().expected, Mode::Pvp)
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
                    .to_statistic(&ctx.data().expected, Mode::Pvp)
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
        .send(
            CreateReply::default()
                .attachment(CreateAttachment::bytes(img, "image.png"))
                .components(view.build())
                .reply(true),
        )
        .await?
        .into_message()
        .await?;
    typing.stop();

    // waiting for interactions
    while let Some(interaction) = msg
        .await_component_interactions(ctx)
        .timeout(Duration::from_secs(60))
        .author_id(ctx.author().id)
        .stream()
        .next()
        .await
    {
        let _typing = ctx.typing().await;

        match interaction.data.custom_id.as_str() {
            "overall_tier" => {
                // disable button first
                view.by_tier_btn_disabled = true;
                let _ok = interaction
                    .create_response(ctx, CreateInteractionResponse::Acknowledge)
                    .await;
                // generate then send image
                let img_tier = view.overall_data.render_tiers(&ctx.data().client).await?;
                let _ok = msg
                    .edit(
                        ctx,
                        EditMessage::new()
                            .attachments(
                                EditAttachments::keep_all(&msg)
                                    .add(CreateAttachment::bytes(img_tier, "image_tier.png")),
                            )
                            .components(view.build()),
                    )
                    .await;
            }
            "overall_cw" => {
                // disable button first
                view.cw_btn_disabled = true;
                let _ok = interaction
                    .create_response(ctx, CreateInteractionResponse::Acknowledge)
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
                let img_cw = overall_cw_data.render(&ctx.data().client).await?;
                let _ok = msg
                    .edit(
                        ctx,
                        EditMessage::new()
                            .attachments(
                                EditAttachments::keep_all(&msg)
                                    .add(CreateAttachment::bytes(img_cw, "image_cw.png")),
                            )
                            .components(view.build()),
                    )
                    .await;
            }
            _ => {}
        }
    }
    // timeout;
    msg.edit(ctx, EditMessage::new().components(view.timeout().build()))
        .await?;
    Ok(())
}

struct WwsView {
    pub overall_data: OverallTemplate,
    pub player: PartialPlayer,
    by_tier_btn_disabled: bool,
    cw_btn_disabled: bool,
}

impl WwsView {
    fn new(overall_data: OverallTemplate, player: PartialPlayer) -> Self {
        Self {
            overall_data,
            player,
            by_tier_btn_disabled: false,
            cw_btn_disabled: false,
        }
    }

    fn build(&self) -> Vec<CreateActionRow> {
        vec![CreateActionRow::Buttons(vec![
            CreateButton::new("overall_tier")
                .style(ButtonStyle::Secondary)
                .label("stats by tier")
                .disabled(self.by_tier_btn_disabled),
            CreateButton::new("overall_cw")
                .style(ButtonStyle::Secondary)
                .label("CB seasons")
                .disabled(self.cw_btn_disabled),
            CreateButton::new_link(self.player.profile_url()).label("Official"),
            CreateButton::new_link(self.player.wows_number_url()).label("Stats & Numbers"),
        ])]
    }

    fn timeout(&mut self) -> &Self {
        self.by_tier_btn_disabled = true;
        self.cw_btn_disabled = true;
        self
    }
}
