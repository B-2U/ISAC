use std::time::{Duration, SystemTime, UNIX_EPOCH};

use itertools::Itertools;
use poise::{
    serenity_prelude::{
        ComponentInteractionDataKind, CreateActionRow, CreateAttachment, CreateInteractionResponse,
        CreateInteractionResponseMessage, CreateSelectMenu, CreateSelectMenuOption,
        EditAttachments, EditMessage, Message, User,
    },
    CreateReply,
};

use crate::{
    dc_utils::{autocomplete, Args, ContextAddon, UserAddon},
    structs::{
        AutocompletePlayer, Mode, PartialPlayer, Player, PlayerSnapshots, PlayerSnapshotsType,
        Ship, ShipId, ShipModeStatsPair, ShipStatsCollection,
    },
    template_data::{
        RecentTemplate, RecentTemplateDiv, RecentTemplateShip, Render, SingleShipTemplate,
    },
    utils::{wws_api::WowsApi, IsacError, IsacInfo},
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
#[poise::command(slash_command, user_cooldown = 3)]
pub async fn recent(
    ctx: Context<'_>,
    #[description = "last 1~30 (90 for patreons) days of stats, default: 1"] days: Option<u64>,
    #[description = "player's ign, default: yourself"]
    #[autocomplete = "autocomplete::player"]
    player: Option<AutocompletePlayer>, // the String is a Serialized PartialPlayer struct
    #[description = "@ping / discord user's ID, default: yourself"]
    #[rename = "user"]
    discord_user: Option<String>,
    #[description = "specific warship, default: all ships' recent"]
    #[rename = "warship"]
    #[autocomplete = "autocomplete::ship"]
    ship_name: Option<String>,
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
            .await
            .unwrap_or(ctx.author().clone())
        } else {
            ctx.author().clone()
        };
        ctx.data()
            .link
            .read()
            .await
            .get(&user.id)
            .ok_or(IsacError::Info(IsacInfo::UserNotLinked {
                user_name: Some(user.name.clone()),
            }))?
    };
    // keep None as None, and raise Err if ship_id is Some(), but no matched ship found
    let ship = ship_name
        .map(|ship_name| {
            ctx.data()
                .ships
                .read()
                .search_name(&ship_name, 1)
                .map(|v| v.first())
        })
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
    let player = partial_player.full_player(&api).await?;
    let filter = specific_ship
        .as_ref()
        .map(|ship| |ship_id: &ShipId, _v: &mut ShipModeStatsPair| ship_id == &ship.ship_id);

    // let filter: Box<dyn Fn(&ShipId, &mut ShipModeStatsPair) -> bool + Send + Sync> =
    //     if let Some(ship) = specific_ship.as_ref() {
    //         Box::new(move |k, _v| k == &ship.ship_id)
    //     } else {
    //         Box::new(|_k, _v| true)
    //     };

    let mut current_ships = partial_player.all_ships(&api).await?;
    let (is_new, is_active, player_data) = load_player(&player, &current_ships).await;
    if let Some(f) = filter {
        current_ships.0.retain(f);
    }
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
        specific_ship.as_ref(), // borrow the ship to prevent clone()
        is_active,
    );

    typing1.stop();
    // getting the stats diff compared with history
    let (exact_day, stats) = loop {
        if target_day > max_day {
            Err(IsacError::Info(IsacInfo::NeedPremium {
                msg: format!("**{target_day}** is illegal, min: **1** max: **{max_day}**"),
            }))?
        }
        let target_time = now - target_day * 86400;
        if let Some((exact_time, stats)) =
            player_data
                .get_date(&target_time)
                .await
                .and_then(|(exact_time, mut old_ships)| {
                    if let Some(f) = filter {
                        old_ships.0.retain(f);
                    }
                    current_ships
                        .compare(old_ships)
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
                ask_struct.finished().await?;
                Err(IsacError::Cancelled)?
            }
        }
    };
    // got the history, constructing template data
    let _typing2 = ctx.typing().await;
    // parsing and render
    let expected = &ctx.data().expected;
    let clan = player.clan(&api).await.ok();
    // QA 這個超大的if else感覺好糟...
    let img = if let Some(ship) = specific_ship.as_ref() {
        // recent ship
        let ship_stats = stats
            .get_ship(&ship.ship_id)
            .expect("it should not be None");

        let data = SingleShipTemplate::new(
            ctx,
            ship.clone(),
            None,
            format!("({} days) {}", exact_day, mode.render_name()),
            ship_stats,
            mode,
            clan,
            player,
        )?;
        data.render(&ctx.data().client).await?
    } else {
        // recent all
        let div = RecentTemplateDiv {
            pvp: stats.to_statistic(expected, Mode::Pvp),
            pvp_solo: stats.to_statistic(expected, Mode::Solo),
            pvp_div2: stats.to_statistic(expected, Mode::Div2),
            pvp_div3: stats.to_statistic(expected, Mode::Div3),
            rank_solo: stats.to_statistic(expected, Mode::Rank),
        };
        let ships = stats
            .0
            .into_iter()
            .filter_map(|(ship_id, ship_stats)| {
                ship_stats
                    .to_statistic(&ship_id, expected, mode)
                    .map(|stats| RecentTemplateShip {
                        info: ship_id.get_ship(&ctx.data().ships).unwrap_or_default(),
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

    let att = CreateAttachment::bytes(img, "image.png");
    if let Some(mut msg) = ask_struct.ask_msg {
        msg.edit(
            ctx,
            EditMessage::default()
                .components(vec![])
                .attachments(EditAttachments::new().add(att)),
        )
        .await?
    } else {
        let _msg = ctx
            .send(CreateReply::default().attachment(att).reply(true))
            .await?;
    }

    Ok(())
}
/// load player data, update the last_requst timestamp, put in current_ships if needed
async fn load_player(
    player: &Player,
    curren_ships: &ShipStatsCollection,
    // QA 打包成一個struct會比較好嗎? (下面PlayerSnapshotsLoadResult)， 但還是得攤開，而且攤開可以利用unused強迫處理
) -> (bool, bool, PlayerSnapshots) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let (is_new, mut player_data) =
        if let Some(player_data) = PlayerSnapshots::load(player.partial_player).await {
            (false, player_data)
        } else {
            let player_data = PlayerSnapshots::init(player.partial_player).await;
            (true, player_data)
        };
    let is_active = match player_data.last_request {
        PlayerSnapshotsType::Premium => true,
        PlayerSnapshotsType::Normal(timestamp) => {
            now - timestamp < RECENT_LAST_REQUEST_LIMIT * 86400
        }
    };
    // update the last_requst timestamp
    player_data.update_last_request(player.premium);
    // put in current_ships if needed
    if now - player_data.last_update_at > 86400 {
        player_data.insert(curren_ships.clone());
    }
    player_data.save().await;

    (is_new, is_active, player_data)
}

// pub struct PlayerSnapshotsLoadResult {
//     pub is_new: bool,    // if its just init
//     pub is_active: bool, // if ISAC is still tracking
//     pub data: PlayerSnapshots,
// }

pub struct AskDay<'a> {
    pub ctx: &'a Context<'a>,
    pub ign: String,
    pub mode: Mode,
    pub ask_msg: Option<Message>,
    pub is_active: bool,
    pub ship: Option<&'a Ship>,
}

impl<'a> AskDay<'a> {
    pub fn new(
        ctx: &'a Context<'_>,
        ign: String,
        mode: Mode,
        ship: Option<&'a Ship>,
        is_active: bool,
    ) -> Self {
        Self {
            ctx,
            ign,
            mode,
            ask_msg: None,
            is_active,
            ship,
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
                (false, vec![])
            } else {
                let options = available_days
                    .iter()
                    .sorted()
                    .dedup()
                    .take(25)
                    .map(|day| CreateSelectMenuOption::new(day.to_string(), day.to_string()))
                    .collect_vec();
                let view = vec![CreateActionRow::SelectMenu(
                    CreateSelectMenu::new(
                        "recent_select",
                        poise::serenity_prelude::CreateSelectMenuKind::String { options },
                    )
                    .min_values(1)
                    .max_values(1),
                )];

                (true, view)
            }
        };
        let msg_content = {
            let specific_ship = self
                .ship
                .as_ref()
                .map_or_else(|| "".to_string(), |ship| format!(" in **{}**", ship.name));
            let mut content = format!(
                "`{}` played 0 battle{specific_ship} in **{}** last **{}** day.",
                self.ign,
                self.mode.upper(),
                current_day
            );
            if has_choices {
                content.push_str("\nChoose an older date?");
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
                .send({
                    CreateReply::default()
                        .content(msg_content)
                        .reply(true)
                        .components(if has_choices { view } else { vec![] })
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
                .edit(self.ctx, {
                    EditMessage::default()
                        .content(msg_content)
                        .components(if has_choices { view } else { vec![] })
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
                .create_response(
                    self.ctx,
                    CreateInteractionResponse::UpdateMessage(
                        CreateInteractionResponseMessage::default(),
                    ),
                )
                .await;
            Some(match &interaction.data.kind {
                ComponentInteractionDataKind::StringSelect { values } => values[0]
                    .parse::<u64>()
                    .expect("it was the day number we provided"),
                _ => unreachable!(),
            })
        } else {
            None
        };
        Ok(response)
    }

    /// remove the components under the ask_msg, this is for the situation that user didn't response
    pub async fn finished(&mut self) -> Result<&AskDay<'a>, Error> {
        if let Some(ask_msg) = self.ask_msg.as_mut() {
            ask_msg
                .edit(self.ctx, EditMessage::default().components(vec![]))
                .await?;
        };
        Ok(self)
    }
}
