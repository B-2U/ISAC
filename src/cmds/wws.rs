use std::{borrow::Cow, collections::HashMap, time::Duration};

use poise::serenity_prelude::{AttachmentType, ButtonStyle, CreateActionRow, CreateButton, User};

use crate::{
    dc_utils::{auto_complete, Args, ContextAddon, InteractionAddon, UserAddon},
    utils::{
        structs::{
            template_data::{
                OverallData,
                OverallDataClass,
                OverallDataDiv,
                OverallDataTier,
                Render, // Render trait一定要在這裡也 use 嗎?
                SingleShipData,
                SingleShipDataSub,
            },
            Linked, Mode, PartialPlayer, Ship, ShipClass, ShipId, ShipLeaderboard, ShipTier,
            Statistic,
        },
        IsacError, IsacInfo,
    },
    Context, Error,
};

/// account / warship stats
#[poise::command(slash_command)] // , rename = "wws"
pub async fn wws_slash(
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
    let partial_player = if let Some(player) = player
        .map(|player_str| serde_json::from_str::<PartialPlayer>(&player_str).ok())
        .flatten()
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
        let mut linked_js: HashMap<_, _> = Linked::load().await.into();
        linked_js
            .remove(&user.id)
            .ok_or(IsacError::Info(IsacInfo::UserNotLinked {
                msg: format!("**{}** haven't linked to any wows account yet", user.name),
            }))?
    };

    if let Some(ship_id) = ship_id {
        // wws ship
        let Some(ship) = ShipId(ship_id).get_ship(&ctx) else {
            let _r = ctx.send(|b|b.content("plz choose a ship in the searching results").ephemeral(true)).await;
            return Ok(())
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
pub async fn wws(ctx: Context<'_>, #[rest] args: Option<Args>) -> Result<(), Error> {
    let mut args = args.unwrap_or_default();
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
    let player = partial_player.get_player(&ctx).await?;
    let clan = player.clan(&ctx).await?;
    let ships = player.single_ship(&ctx, &ship).await?;
    let stats = ships.to_statistic(&ctx.data().expected_js, mode);
    if stats.battles == 0 {
        Err(IsacError::Info(IsacInfo::PlayerNoBattleShip {
            ign: player.ign.clone(),
            ship_name: ship.name.clone(),
            region: player.region,
        }))?
    };
    let sub_modes = if let Mode::Rank = mode {
        None
    } else {
        Some(SingleShipDataSub::new(
            ships.to_statistic(&ctx.data().expected_js, Mode::Solo),
            ships.to_statistic(&ctx.data().expected_js, Mode::Div2),
            ships.to_statistic(&ctx.data().expected_js, Mode::Div3),
        ))
    };
    // getting player rank in the leaderboard
    let rank = if let Ok(leaderboard) = ShipLeaderboard::load(player.region).await {
        leaderboard
            .0
            .get(&ship.ship_id)
            .and_then(|ship| ship.players.iter().find(|p| p.uid == player.uid))
            .map(|p| p.rank)
    } else {
        None
    };

    let data = SingleShipData {
        ship,
        rank,
        main_mode_name: mode.display_name(),
        main_mode: stats,
        sub_modes,
        clan,
        user: player,
    };
    // TODO: test if ranking work after the .top command finished
    let img = data.render(&ctx.data().client).await?;
    let _msg = ctx
        .send(|b| {
            b.attachment(poise::serenity_prelude::AttachmentType::Bytes {
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

async fn func_wws(ctx: &Context<'_>, partial_player: PartialPlayer) -> Result<(), Error> {
    let typing = ctx.typing().await;
    let player = partial_player.get_player(&ctx).await?;
    let clan = player.clan(&ctx).await?;

    // wws
    let ships = player.all_ships(&ctx).await?;
    let div = OverallDataDiv::new(
        ships.to_statistic(&ctx.data().expected_js, Mode::Pvp),
        ships.to_statistic(&ctx.data().expected_js, Mode::Solo),
        ships.to_statistic(&ctx.data().expected_js, Mode::Div2),
        ships.to_statistic(&ctx.data().expected_js, Mode::Div3),
    );
    let class: OverallDataClass = ships
        .clone()
        .sort_class(&ctx)
        .into_iter()
        .map(|(class, ships)| {
            (
                class,
                ships.to_statistic(&ctx.data().expected_js, Mode::Pvp),
            )
        })
        .collect::<HashMap<ShipClass, Statistic>>()
        .into();
    let tier: OverallDataTier = ships
        .sort_tier(&ctx)
        .into_iter()
        .map(|(class, ships)| {
            (
                class,
                ships.to_statistic(&ctx.data().expected_js, Mode::Pvp),
            )
        })
        .collect::<HashMap<ShipTier, Statistic>>()
        .into();
    let overall_data = OverallData {
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
            b.attachment(poise::serenity_prelude::AttachmentType::Bytes {
                data: Cow::Borrowed(&img),
                filename: "image.png".to_string(),
            })
            .components(|c| c.set_action_row(view.build()))
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
        let _tpying2 = ctx.typing().await;
        let img_2 = overall_data.render_tiers(&ctx.data().client).await?;
        // disable button first
        let _ok = interaction
            .edit_original_message(ctx, |m| {
                m.components(|c| c.set_action_row(view.timout().build()))
            })
            .await;
        let _ok = msg
            .edit(ctx, |m| {
                m.attachment(AttachmentType::Bytes {
                    data: Cow::Borrowed(&img_2),
                    filename: "image.png".to_string(),
                })
                .components(|c| c.set_action_row(view.timout().build()))
            })
            .await;
    } else {
        // timeout disable button
        let _ok = msg
            .edit(ctx, |m| {
                m.components(|c| c.set_action_row(view.timout().build()))
            })
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

    fn build(&self) -> CreateActionRow {
        CreateActionRow::default()
            .add_button(self.by_tier_btn.clone())
            .create_button(|b| {
                b.label("Official")
                    .url(self.player.profile_url().unwrap())
                    .style(ButtonStyle::Link)
            })
            .create_button(|b| {
                b.label("Stats & Numbers")
                    .url(self.player.wows_number_url().unwrap())
                    .style(ButtonStyle::Link)
            })
            .to_owned()
    }

    fn timout(&mut self) -> &Self {
        self.by_tier_btn.disabled(true);
        self
    }
}
