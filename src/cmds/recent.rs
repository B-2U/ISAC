use std::{
    borrow::Cow,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use itertools::Itertools;
use poise::serenity_prelude::{
    AttachmentType, CreateComponents, CreateSelectMenuOption, Message, User,
};

use crate::{
    cmds::recent,
    dc_utils::{auto_complete, Args, ContextAddon, InteractionAddon, UserAddon},
    utils::{
        structs::{
            template_data::{RecentTemplate, RecentTemplateDiv, RecentTemplateShip, Render},
            Mode, PartialPlayer, Player, RecentPlayer, RecentPlayerType, ShipStatsCollection,
        },
        IsacError, IsacInfo,
    },
    Context, Data, Error,
};

const RECENT_LAST_REQUEST_LIMIT: u64 = 14;
const RECENT_OMIT_LIMIT: usize = 50;

pub fn recent_hybrid() -> poise::Command<Data, Error> {
    let mut cmd = recent::recent_slash();
    cmd.prefix_action = recent::recent().prefix_action;
    cmd
}

/// Last X days stats
#[poise::command(slash_command, rename = "recent")]
pub async fn recent_slash(
    ctx: Context<'_>,
    #[description = "player's ign, default: yourself"]
    #[autocomplete = "auto_complete::player"]
    player: Option<String>, // the String is a Serialized PartialPlayer struct
    #[description = "@ping / discord user's ID, default: yourself"]
    #[rename = "user"]
    discord_user: Option<String>,
    #[description = "last 1~30(90 for patreons) days of stats, default: 1"] days: Option<u64>,
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

    func_recent(
        &ctx,
        partial_player,
        battle_type.unwrap_or_default(),
        days.unwrap_or(1),
    )
    .await
}

#[poise::command(prefix_command)]
pub async fn recent(ctx: Context<'_>, #[rest] mut args: Args) -> Result<(), Error> {
    let partial_player = args.parse_user(&ctx).await?;
    let mode = args.parse_mode().unwrap_or_default();
    let day = args.parse_day().unwrap_or(1);
    // TODO new feature, parse_ship() here?
    func_recent(&ctx, partial_player, mode, day).await
}

async fn func_recent(
    ctx: &Context<'_>,
    partial_player: PartialPlayer,
    mode: Mode,
    day: u64,
) -> Result<(), Error> {
    let typing1 = ctx.typing().await;
    let max_day = match ctx.data().patron.read().check_user(&ctx.author().id) {
        true => 91_u64, // well its actually 90, this is for some 90 days data get ceiling to 91
        false => 30_u64,
    };
    let player = partial_player.get_player(ctx).await?;
    let current_ships = partial_player.all_ships(ctx).await?;
    let (is_new, is_active, player_data) = load_player(&player, &current_ships).await;
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
    let mut ask_struct = AskDay::new(ctx, player.ign.clone(), mode, is_active);

    typing1.stop();
    // getting available history
    let (exact_time, stats) = loop {
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
                    current_ships
                        .compare(old_ships)
                        .map(|stats| (exact_time, stats))
                })
        {
            break (exact_time, stats);
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
        clan: player.clan(ctx).await?,
        user: player,
        ships,
        day: ((now - exact_time) as f64 / 86400.0).ceil() as u64,
        main_mode_name: mode.render_name().to_string(),
        main: div.get_mode(&mode).cloned().unwrap_or_default(),
        div,
    };
    let img = data.render(&ctx.data().client).await?;
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
/// laod player data, update the last_requst timestamp, put in current_ships if needed
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
}

impl<'a> AskDay<'a> {
    pub fn new(ctx: &'a Context<'_>, ign: String, mode: Mode, is_active: bool) -> Self {
        Self {
            ctx,
            ign,
            mode,
            ask_msg: None,
            is_active,
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
            let mut content = format!(
                "`{}` played 0 battle in **{}** last **{}** day.",
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
            let _r = interaction.edit_original_message(self.ctx, |m| m).await;
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
