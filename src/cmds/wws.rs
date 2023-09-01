use std::{borrow::Cow, collections::HashMap, time::Duration};

use poise::serenity_prelude::{
    AttachmentType, ButtonStyle, CreateActionRow, CreateButton, CreateComponents, User,
};

use crate::{
    dc_utils::{auto_complete, Args, ContextAddon, CreateReplyAddon, InteractionAddon, UserAddon},
    utils::{
        structs::{
            template_data::{
                OverallTemplate,
                OverallTemplateClass,
                OverallTemplateDiv,
                OverallTemplateTier,
                Render, // Render trait一定要在這裡也 use 嗎?
                SingleShipTemplate,
                SingleShipTemplateSub,
            },
            Mode, PartialPlayer, Ship, ShipClass, ShipId, ShipTier, Statistic,
        },
        wws_api::WowsApi,
        IsacError, IsacInfo,
    },
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
#[poise::command(slash_command)]
pub async fn wws(
    ctx: Context<'_>,
    #[description = "specific warship, default: account's overall stats"]
    #[rename = "warship"]
    #[autocomplete = "auto_complete::ship"]
    ship_id: Option<u64>,
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
            .get(&user.id)
            .ok_or(IsacError::Info(IsacInfo::UserNotLinked {
                user_name: Some(user.name.clone()),
            }))?
    };

    if let Some(ship_id) = ship_id {
        // wws ship
        let Some(ship) = ShipId(ship_id).get_ship(&ctx.data().ship_js) else {
            Err(IsacError::Info(IsacInfo::AutoCompleteError))?
        };
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
    let player = partial_player.get_player(&api).await?;
    let clan = player.clan(&api).await?;
    let (ship_id, ship_stats) = player.single_ship(&api, &ship).await?.unwrap_or_default(); // let it default, we will raise error belowed
    let Some(stats) = ship_stats.to_statistic(&ship_id, &ctx.data().expected_js, mode) else {
        Err(IsacError::Info(IsacInfo::PlayerNoBattleShip {
            ign: player.ign.clone(),
            ship_name: ship.name.clone(),
            mode,
        }))?
    };
    let sub_modes = if let Mode::Rank = mode {
        None
    } else {
        Some(SingleShipTemplateSub::new(
            ship_stats
                .to_statistic(&ship_id, &ctx.data().expected_js, Mode::Solo)
                .unwrap_or_default(),
            ship_stats
                .to_statistic(&ship_id, &ctx.data().expected_js, Mode::Div2)
                .unwrap_or_default(),
            ship_stats
                .to_statistic(&ship_id, &ctx.data().expected_js, Mode::Div3)
                .unwrap_or_default(),
        ))
    };
    // getting player rank in the leaderboard
    let ranking = ctx
        .data()
        .leaderboard
        .lock()
        .expect("lb poisoned!")
        .get_ship(&player.region, &ship.ship_id, false)
        .and_then(|players| {
            players
                .into_iter()
                .find(|p| p.uid == player.uid)
                .map(|p| p.rank)
        });

    let data = SingleShipTemplate {
        ship,
        ranking,
        suffix: mode.render_name().to_string(),
        main_mode: stats,
        sub_modes,
        clan,
        user: player,
    };
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
    let player = partial_player.get_player(&api).await?;
    let clan = player.clan(&api).await?;

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
        .sort_class(&ctx)
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
        .sort_tier(&ctx)
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
        clan,
        user: player,
    };
    let img = overall_data.render(&ctx.data().client).await?;
    let mut view = WwsView::new(partial_player);
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
    // waiting for by tier btn
    if let Some(interaction) = msg
        .await_component_interaction(ctx)
        .timeout(Duration::from_secs(60))
        .author_id(ctx.author().id)
        .await
    {
        let _typing = ctx.typing().await;
        let img_2 = overall_data.render_tiers(&ctx.data().client).await?;
        // disable button first
        let _ok = interaction
            .edit_original_message(ctx, |m| m.set_components(view.timout().build()))
            .await;
        let _ok = msg
            .edit(ctx, |m| {
                m.attachment(AttachmentType::Bytes {
                    data: Cow::Borrowed(&img_2),
                    filename: "image.png".to_string(),
                })
                .set_components(view.timout().build())
            })
            .await;
    } else {
        // timeout disable button
        let _ok = msg
            .edit(ctx, |m| m.set_components(view.timout().build()))
            .await;
    }
    Ok(())
}

struct WwsView {
    pub player: PartialPlayer,
    by_tier_btn: CreateButton,
}

impl WwsView {
    fn new(player: PartialPlayer) -> Self {
        let btn = CreateButton::default()
            .custom_id("overall_tier")
            .style(poise::serenity_prelude::ButtonStyle::Secondary)
            .label("stats by tier")
            .to_owned();
        Self {
            player,
            by_tier_btn: btn,
        }
    }

    fn build(&self) -> CreateComponents {
        let mut view = CreateComponents::default();
        let mut row = CreateActionRow::default();
        row.add_button(self.by_tier_btn.clone())
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

    fn timout(&mut self) -> &Self {
        self.by_tier_btn.disabled(true);
        self
    }
}
