use std::{
    borrow::Cow,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use itertools::Itertools;
use poise::serenity_prelude::{
    AttachmentType, CreateComponents, CreateSelectMenuOption, Message, User,
};

use crate::{
    dc_utils::{auto_complete, Args, ContextAddon, InteractionAddon, UserAddon},
    utils::{
        structs::{
            template_data::{
                RecentTemplate, RecentTemplateDiv, RecentTemplateShip, Render, SingleShipTemplate,
                SingleShipTemplateSub,
            },
            Mode, PartialPlayer, Player, RecentPlayer, RecentPlayerType, Ship, ShipId,
            ShipModeStatsPair, ShipStatsCollection,
        },
        wws_api::WowsApi,
        IsacError, IsacInfo,
    },
    Context, Data, Error,
};

const RECENT_LAST_REQUEST_LIMIT: u64 = 14;
const RECENT_OMIT_LIMIT: usize = 50;

pub fn recent_hybrid() -> poise::Command<Data, Error> {
    poise::Command {
        prefix_action: recent_prefix().prefix_action,
        slash_action: recent().slash_action,
        ..recent()
    }
}

/// Last X days stats
#[poise::command(slash_command)]
pub async fn recent(
    ctx: Context<'_>,
    #[description = "player's ign, default: yourself"]
    #[autocomplete = "auto_complete::player"]
    player: Option<String>, // the String is a Serialized PartialPlayer struct
    #[description = "@ping / discord user's ID, default: yourself"]
    #[rename = "user"]
    discord_user: Option<String>,
    #[description = "last 1~30(90 for patreons) days of stats, default: 1"] days: Option<u64>,
    #[description = "specific warship, default: all ships' recent"]
    #[rename = "warship"]
    #[autocomplete = "auto_complete::ship"]
    ship_id: Option<u64>,
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
    // keep None as None, and raise Err if ship_id is Some(), but no matched ship found
    let ship = ship_id
        .map(
            |ship_id| match ShipId(ship_id).get_ship(&ctx.data().ship_js) {
                Some(ship) => Ok(ship),
                None => Err(IsacError::Info(IsacInfo::AutoCompleteError)),
            },
        )
        .transpose()?;

    func_recent(
        &ctx,
        partial_player,
        battle_type.unwrap_or_default(),
        days.unwrap_or(1),
        ship,
    )
    .await
}

#[poise::command(prefix_command)]
pub async fn recent_prefix(ctx: Context<'_>, #[rest] mut args: Args) -> Result<(), Error> {
    let partial_player = args.parse_user(&ctx).await?;
    let mode = args.parse_mode().unwrap_or_default();
    let day = args.parse_day().unwrap_or(1);
    let ship = if !args.is_empty() {
        // specific ship
        Some(args.parse_ship(&ctx).await?)
    } else {
        None
    };

    func_recent(&ctx, partial_player, mode, day, ship).await
}

async fn func_recent(
    ctx: &Context<'_>,
    partial_player: PartialPlayer,
    mode: Mode,
    day: u64,
    specific_ship: Option<Ship>,
) -> Result<(), Error> {
    let typing1 = ctx.typing().await;
    let api = WowsApi::new(ctx);
    let max_day = match ctx.data().patron.read().check_user(&ctx.author().id) {
        true => 91_u64, // well its actually 90, this is for some 90 days data get ceiling to 91
        false => 30_u64,
    };
    let player = partial_player.get_player(&api).await?;
    // QA making a custom filter, better way...? and I have to call as_ref() beloweds
    let filter: Box<dyn Fn(&ShipId, &mut ShipModeStatsPair) -> bool + Send + Sync> =
        if let Some(ship) = specific_ship.as_ref() {
            Box::new(move |k, _v| k == &ship.ship_id)
        } else {
            Box::new(|_k, _v| true)
        };

    let current_ships = partial_player.all_ships(&api).await?;
    let (is_new, is_active, player_data) = load_player(&player, &current_ships).await;
    let current_ships_filted = current_ships.retain(filter.as_ref());
    if is_new {
        let msg = format!(
            "`{}` wasn't in the database, please play a game then try the command again",
            player.ign
        );
        Err(IsacError::Info(IsacInfo::GeneralError { msg }))?
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut target_day = day;
    let mut ask_struct = AskDay::new(
        ctx,
        player.ign.clone(),
        mode,
        specific_ship.as_ref().map(|s| s.name.clone()),
        is_active,
    );

    typing1.stop();
    // getting the stats diff compared with history
    let (exact_day, mut stats) = loop {
        if target_day > max_day {
            Err(IsacError::Info(IsacInfo::NeedPremium {
                msg: format!("**{target_day}** is illegal, min: **1** max:**{max_day}**"),
            }))?
        }
        let target_time = now - target_day * 86400;
        if let Some((exact_time, stats)) =
            player_data
                .get_date(&target_time)
                .await
                .and_then(|(exact_time, old_ships)| {
                    current_ships_filted
                        .compare(old_ships.retain(filter.as_ref()))
                        .map(|stats| (exact_time, stats))
                })
        {
            break (((now - exact_time) as f64 / 86400.0).ceil() as u64, stats);
        } else {
            // no data or the same, ask user to re-select
            let available_times = player_data.available_dates(&target_time);
            let available_days = available_times
                .iter()
                .map(|t| ((now - *t) as f64 / 86400.0).ceil() as u64)
                .collect_vec();
            // pick date here
            if let Some(selected_day) = ask_struct.ask(target_day, available_days).await? {
                // next iter
                target_day = selected_day;
            } else {
                ask_struct.finished(None::<String>).await?;
                Err(IsacError::Cancelled)?
            }
        }
    };
    let _typing2 = ctx.typing().await;
    // parsing and render
    let expected_js = &ctx.data().expected_js;
    let clan = player.clan(&api).await?;
    // QA 這個超大的if else感覺好糟...
    let img = if let Some(ship) = specific_ship.as_ref() {
        // recent ship
        let ship_stats = stats
            .0
            .remove(&ship.ship_id)
            .expect("it should not be None");
        let Some(main_mode) = ship_stats.to_statistic(&ship.ship_id, &ctx.data().expected_js, mode) else {
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
                    .to_statistic(&ship.ship_id, expected_js, Mode::Solo)
                    .unwrap_or_default(),
                ship_stats
                    .to_statistic(&ship.ship_id, expected_js, Mode::Div2)
                    .unwrap_or_default(),
                ship_stats
                    .to_statistic(&ship.ship_id, expected_js, Mode::Div3)
                    .unwrap_or_default(),
            ))
        };
        let data = SingleShipTemplate {
            ship: ship.clone(),
            ranking: None,
            suffix: format!("({} days) {}", exact_day, mode.render_name()),
            main_mode,
            sub_modes,
            clan,
            user: player,
        };
        data.render(&ctx.data().client).await?
    } else {
        // recent all
        let div = RecentTemplateDiv {
            pvp: stats.to_statistic(expected_js, Mode::Pvp),
            pvp_solo: stats.to_statistic(expected_js, Mode::Solo),
            pvp_div2: stats.to_statistic(expected_js, Mode::Div2),
            pvp_div3: stats.to_statistic(expected_js, Mode::Div3),
            rank_solo: stats.to_statistic(expected_js, Mode::Rank),
        };
        let ships = stats
            .0
            .into_iter()
            .filter_map(|(ship_id, ship_stats)| {
                ship_stats
                    .to_statistic(&ship_id, expected_js, mode)
                    .map(|stats| RecentTemplateShip {
                        info: ship_id.get_ship(&ctx.data().ship_js).unwrap_or_default(),
                        stats,
                    })
            })
            .sorted_by_key(|ship| (-(ship.stats.battles as i64), -(ship.info.tier as i64)))
            .take(RECENT_OMIT_LIMIT)
            .collect_vec();

        let data = RecentTemplate {
            clan,
            user: player,
            ships,
            day: exact_day,
            suffix: mode.render_name().to_string(),
            main: div.get_mode(&mode).cloned().unwrap_or_default(),
            div,
        };
        data.render(&ctx.data().client).await?
    };
    let attachment = AttachmentType::Bytes {
        data: Cow::Borrowed(&img),
        filename: "image.png".to_string(),
    };
    if let Some(mut msg) = ask_struct.ask_msg {
        msg.edit(ctx, |m| {
            m.set_components(CreateComponents::default())
                .content("")
                .attachment(attachment)
        })
        .await?
    } else {
        let _msg = ctx.send(|b| b.attachment(attachment).reply(true)).await?;
    }

    Ok(())
}
/// load player data, update the last_requst timestamp, put in current_ships if needed
async fn load_player(
    player: &Player,
    curren_ships: &ShipStatsCollection,
    // QA 打包成一個struct會比較好嗎? (下面RecentPlayerLoadResult)， 但還是得攤開，而且攤開可以利用unused強迫處理
) -> (bool, bool, RecentPlayer) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let last_request = if player.premium {
        RecentPlayerType::Premium
    } else {
        RecentPlayerType::Normal(now)
    };
    let (is_new, mut player_data) =
        if let Some(player_data) = RecentPlayer::load(&player.partial_player).await {
            (false, player_data)
        } else {
            let player_data = RecentPlayer::init(&player.partial_player).await;
            (true, player_data)
        };
    let is_active = match player_data.last_request {
        RecentPlayerType::Premium => true,
        RecentPlayerType::Normal(timestamp) => now - timestamp < RECENT_LAST_REQUEST_LIMIT * 86400,
    };
    // update the last_requst timestamp
    player_data.last_request = last_request;
    // put in current_ships if needed
    if now - player_data.last_update_at > 86400 {
        player_data.data.insert(now, curren_ships.clone());
    }
    player_data.save().await;

    (is_new, is_active, player_data)
}

// pub struct RecentPlayerLoadResult {
//     pub is_new: bool,    // if its just init
//     pub is_active: bool, // if ISAC is still tracking
//     pub data: RecentPlayer,
// }

pub struct AskDay<'a> {
    pub ctx: &'a Context<'a>,
    pub ign: String,
    pub mode: Mode,
    pub ask_msg: Option<Message>,
    pub is_active: bool,
    pub ship_name: Option<String>, // ship name
}

impl<'a> AskDay<'a> {
    pub fn new(
        ctx: &'a Context<'_>,
        ign: String,
        mode: Mode,
        ship_name: Option<String>,
        is_active: bool,
    ) -> Self {
        Self {
            ctx,
            ign,
            mode,
            ask_msg: None,
            is_active,
            ship_name,
        }
    }
    /// ask user to select a day, return None if user didn't response
    pub async fn ask(
        &mut self,
        current_day: u64,
        available_days: Vec<u64>,
    ) -> Result<Option<u64>, Error> {
        // setting stuffs up
        // QA 原本單純用view.is_some()判斷，但過程view會被moved，才加了這個 has_choices，有更好的做法?
        let (has_choices, view) = {
            if available_days.is_empty() {
                (false, None)
            } else {
                let mut view = CreateComponents::default();
                let options = available_days
                    .iter()
                    .sorted()
                    .dedup()
                    .take(25)
                    .map(|day| {
                        let mut option = CreateSelectMenuOption::default();
                        option.label(day.to_string()).value(*day);
                        // if index == 0 {
                        //     option.default_selection(true);
                        // }
                        option
                    })
                    .collect_vec();
                view.create_action_row(|r| {
                    r.create_select_menu(|m| {
                        m.placeholder("choose one (´･ω･`)? ")
                            .custom_id("recent_select")
                            .options(|op| op.set_options(options))
                            .max_values(1)
                            .min_values(1)
                    })
                });

                (true, Some(view))
            }
        };
        let msg_content = {
            let specific_ship = self.ship_name.as_ref().map_or_else(
                || "".to_string(),
                |ship_name| format!(" in **{}**", ship_name),
            );
            let mut content = format!(
                "`{}` played 0 battle{specific_ship} in **{}** last **{}** day.",
                self.ign,
                self.mode.upper(),
                current_day
            );
            if has_choices {
                content.push_str("\nChoose a larger day number?");
            }
            if !self.is_active {
                content.push_str("```\nYou didn't use this command in the last 14 days, so ISAC stopped tracking your account, plz play a game and try it again```");
            }
            content
        };
        // sending ask message
        if self.ask_msg.is_none() {
            // first time
            let msg = self
                .ctx
                .send(|b| {
                    if has_choices {
                        b.components = view;
                    }
                    b.content(msg_content).reply(true)
                })
                .await?
                .into_message()
                .await?;
            self.ask_msg = Some(msg);
        } else {
            // edit
            self.ask_msg
                .as_mut()
                .expect("it shouldn't happen")
                .edit(self.ctx, |m| {
                    if has_choices {
                        m.set_components(
                            view.expect("if has_choices == true, view is always Some()"),
                        );
                    }
                    m.content(msg_content)
                })
                .await?;
        }
        if !has_choices {
            Err(IsacError::Cancelled)?
        }
        // wait for response
        let response = if let Some(interaction) = self
            .ask_msg
            .as_ref()
            .expect("msg should be exist")
            .await_component_interaction(self.ctx)
            .timeout(Duration::from_secs(15))
            .author_id(self.ctx.author().id)
            .await
        {
            let _r = interaction
                .edit_original_message(self.ctx, |m| m.set_components(CreateComponents::default()))
                .await;
            Some(
                interaction.data.values[0] // there should be one and only one value because of the min max limits
                    .parse::<u64>()
                    .expect("it was the day number we provided"),
            )
        } else {
            None
        };
        Ok(response)
    }

    /// remove the components under the ask_msg, this is for the situation that user didn't response
    pub async fn finished<D: ToString + Sized>(
        &mut self,
        msg: Option<D>,
    ) -> Result<&AskDay<'a>, Error> {
        if let Some(ask_msg) = self.ask_msg.as_mut() {
            ask_msg
                .edit(self.ctx, |m| {
                    if let Some(msg_content) = msg {
                        m.content(msg_content);
                    };
                    m.components(|c| c)
                })
                .await?;
        };
        Ok(self)
    }
}
