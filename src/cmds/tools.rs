use std::{fmt::Write, sync::Arc};

use chrono::NaiveDate;
use itertools::Itertools;
use once_cell::sync::Lazy;
use poise::{
    CreateReply, FrameworkError,
    futures_util::StreamExt,
    serenity_prelude::{
        ButtonStyle, CreateActionRow, CreateAttachment, CreateButton, CreateEmbed,
        CreateEmbedAuthor, CreateInteractionResponse, CreateInteractionResponseMessage,
        EditMessage, Message, ReactionType, User, UserId,
    },
};
use rand::seq::IndexedRandom;
use regex::Regex;
use scraper::{Element, ElementRef, Html, Selector};

use crate::{
    Context, Data, Error,
    dc_utils::{Args, ContextAddon, EasyEmbed},
    structs::{PartialPlayer, Region, Ship},
    utils::{IsacError, IsacInfo, wws_api::WowsApi},
};

/// The link to wargaming wiki maps page
#[poise::command(prefix_command, slash_command, discard_spare_arguments)]
pub async fn map(ctx: Context<'_>) -> Result<(), Error> {
    let _r = ctx.reply("https://wiki.wargaming.net/en/Ship:Maps").await;
    Ok(())
}

/// Get the player's ign from his uid
#[poise::command(prefix_command)]
pub async fn ign(ctx: Context<'_>, #[rest] mut args: Args) -> Result<(), Error> {
    let region = args.parse_region(&ctx).await?;
    let uid: u64 = args.check(0)?.parse().map_err(|_| {
        IsacError::Info(IsacInfo::GeneralError {
            msg: "Not a valid uid".to_string(),
        })
    })?;

    let pp = PartialPlayer { region, uid };
    let player = pp.full_player(&WowsApi::new(&ctx)).await?;

    ctx.reply(format!("`{}`", player.ign)).await?;

    Ok(())
}

async fn code_err_handler(err: FrameworkError<'_, Data, Error>) {
    eprintln!("{err}");
    if let Some(ctx) = err.ctx() {
        let _r = ctx
            .reply("`.code <codes>`\ne.g. `.code TIANSUOHAO2 MONSTERSENPAI`")
            .await;
    }
}
/// Generate the link for redeeming wows bonus codes
#[poise::command(
    prefix_command,
    slash_command,
    aliases("bonus"),
    on_error = "code_err_handler"
)]
pub async fn code(
    ctx: Context<'_>,
    #[rest]
    #[description = "one or more codes, split with space or new line"]
    codes: String,
) -> Result<(), Error> {
    if codes.is_empty() {
        let _r = ctx
            .reply("`.code <codes>`\ne.g. `.code TIANSUOHAO2 MONSTERSENPAI`")
            .await;
        return Ok(());
    };
    let codes_vec = codes
        .split_whitespace()
        .filter(|code| code.len() <= 80 && code.chars().all(|c| c.is_ascii_alphanumeric()))
        .collect::<Vec<_>>();

    if codes_vec.len() > 10 {
        Err(IsacInfo::GeneralError {
            msg: "Too many codes, max 10 codes at once".to_string(),
        })?
    }

    let code_buttons: Vec<CreateActionRow> = codes_vec
        .chunks(5)
        .map(|code_chunk| {
            CreateActionRow::Buttons(
                code_chunk
                    .iter()
                    .map(|&code| {
                        CreateButton::new_link(format!(
                            "https://wargaming.net/shop/redeem/?bonus_mode={code}"
                        ))
                        .label(code)
                    })
                    .collect(),
            )
        })
        .collect();
    let msg = ctx
        .send(
            CreateReply::default()
                .content("dorodoro, here are the codes:")
                .components(code_buttons)
                .reply(true),
        )
        .await?
        .into_message()
        .await?;
    let _r = msg
        .react(ctx, ReactionType::Unicode("❤️".to_string()))
        .await;
    Ok(())
}

#[poise::command(prefix_command)]
pub async fn history(ctx: Context<'_>, #[rest] mut args: Args) -> Result<(), Error> {
    let _typing = ctx.typing().await;
    let api = WowsApi::new(&ctx);
    let player = args.parse_user(&ctx).await?;
    // this is just for rasing error when player profile is hidden
    let _ = player.full_player(&api).await?;
    let res_text = api
        .ureq(player.wows_number_url(), |b| b)
        .await?
        .read_to_string()?;
    // all the clans the player have been in
    let record_clans = {
        let html = Html::parse_document(&res_text);

        let transfer_header_selector = Selector::parse(".section-header").unwrap();
        let table_selector = Selector::parse(".table-styled").unwrap();
        let cells_selector = Selector::parse("tr").unwrap();
        let a_selector = Selector::parse("a").unwrap();
        static CLAN_UID_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"/clan/(\d+),").unwrap());

        let transfer_header = html
            .select(&transfer_header_selector)
            // find the `Transfers` title header
            .find(|h| h.text().next().unwrap_or("") == "Transfers")
            // get header's parent
            .and_then(|e| e.parent().and_then(ElementRef::wrap))
            .ok_or(IsacError::Info(IsacInfo::GeneralError {
                msg: "Parsing failed".to_string(),
            }))?;
        // find tables in it
        let tables = transfer_header.select(&table_selector).collect::<Vec<_>>();
        let target_table = match tables.len() {
            0 => Err(IsacError::Info(IsacInfo::GeneralError {
                msg: "No transfer history".to_string(),
            })),
            1 => Ok(tables[0]),
            2 => Ok(tables[1]),
            n => Err(IsacError::Info(IsacInfo::GeneralError {
                msg: format!("Parsing failed, tables.len() = {n}"),
            })),
        }?;
        let cells = target_table.select(&cells_selector);

        cells
            .filter_map(|cell| {
                let selected = cell.select(&a_selector).next()?;
                let clan_name = selected
                    .inner_html()
                    .split_whitespace()
                    .next()
                    .unwrap_or("[]")
                    .to_string();
                let _clan_href = selected.value().attr("href")?;
                CLAN_UID_RE
                    .captures(_clan_href)?
                    .get(1)
                    .and_then(|uid| uid.as_str().parse::<u64>().ok())
                    .map(|i| (i, clan_name))
            })
            .sorted_unstable()
            .dedup_by(|a, b| a.0 == b.0)
            .collect::<Vec<(u64, String)>>()
    };

    let mut name_history = vec![];
    for (clan_uid, clan_tag) in record_clans {
        let res_text = api
            .ureq(
                player
                    .region
                    .number_url(format!("/clan/transfers/{clan_uid},/")),
                |b| b,
            )
            .await?
            .read_to_string()?;
        let transfers = _rename_parse_clan(res_text, &player).map_err(IsacError::UnknownError)?;
        name_history.extend(
            transfers
                .into_iter()
                .map(|(date, ign)| (date, ign, clan_tag.clone())),
        );
    }
    name_history.sort_unstable_by(|a, b| a.0.cmp(&b.0));

    let output = name_history
        .iter()
        .fold(String::new(), |mut buf, (date, ign, clan_tag)| {
            let _ = writeln!(buf, "{ign} {clan_tag}, {}", date.format("%Y.%m.%d"));
            buf
        });
    let output = format!("```py\n{}\n```", output);
    if ctx.reply(&output).await.is_err() {
        // the history might exceed the limit, send it as a file
        let _r = ctx
            .send(
                CreateReply::default()
                    .attachment(CreateAttachment::bytes(output.as_bytes(), "rename history")),
            )
            .await;
    }
    Ok(())
}

/// parsing the clan transfer html, return Vec<(NaiveDate, ign)>
fn _rename_parse_clan(
    html_text: impl AsRef<str>,
    player: &PartialPlayer,
) -> Result<Vec<(NaiveDate, String)>, Error> {
    let html = Html::parse_document(html_text.as_ref());

    let cells_selector = Selector::parse("table tr").unwrap();
    let cells = html.select(&cells_selector);

    static PLAYER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"/player/(\d+),").unwrap());

    let mut history_names: Vec<(NaiveDate, String)> = vec![];

    // the first cell is just header, skip it
    for cell in cells.skip(1) {
        let i_selector = Selector::parse("td").unwrap();
        let i = cell.select(&i_selector).collect::<Vec<_>>();

        // Extract data
        let _player_href = i[2]
            .first_element_child()
            .and_then(|ele| ele.value().attr("href"))
            .unwrap();
        let player_uid = PLAYER_RE
            .captures(_player_href)
            .and_then(|c| c.get(1))
            .unwrap()
            .as_str()
            .parse::<u64>()?;
        // player not matched, continue
        if player_uid != player.uid {
            continue;
        }
        let date_str = i[1].inner_html();
        let player_ign = i[2]
            .first_element_child()
            .ok_or("player_ign first_element")?
            .inner_html();
        let date: NaiveDate = NaiveDate::parse_from_str(&date_str, "%d.%m.%Y")?;
        // let naivedate_epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        // let time_stamp = date.signed_duration_since(naivedate_epoch);
        history_names.push((date, player_ign));
    }

    // println!("{:?}", history_names);
    Ok(history_names)
}

/// Picking ships randomly for you
#[poise::command(slash_command)]
pub async fn roulette(
    ctx: Context<'_>,
    #[description = "how many players in the div? default: 3"] players: Option<RoulettePlayer>,
    #[description = "ships tier, default: 10"] tier: Option<RouletteTier>,
) -> Result<(), Error> {
    let players = players.unwrap_or(RoulettePlayer::Three);
    let tier = tier.unwrap_or(RouletteTier::X);
    let cadidates = {
        let ship_js = ctx.data().ships.read();
        ship_js
            .0
            .iter()
            .filter(|(_ship_id, ship)| ship.tier as u8 == tier as u8 && ship.is_available())
            .map(|(_ship_id, ship)| ship.clone())
            .collect::<Vec<_>>()
    };
    // let mut ships: Vec<Ship> = cadidates
    //     .choose_multiple(&mut rand::thread_rng(), players.to_int())
    //     .map(|&m| m.clone())
    //     .collect();

    let mut view = RouletteView::new(cadidates, players, ctx.author().clone());

    let inter_msg = ctx
        .send(
            CreateReply::default()
                .embed(view.embed_build())
                .components(view.build()),
        )
        .await
        .map_err(|_| IsacError::Info(crate::utils::IsacInfo::EmbedPermission))?
        .into_message()
        .await?;

    let timeout = std::time::Duration::from_secs(60 * 2);
    let _interaction_result = view
        .interactions(&ctx, ctx.author().id, inter_msg, timeout)
        .await;
    Ok(())
}

#[derive(Debug, Clone)]
struct RouletteView {
    players: RoulettePlayer,
    candidates: Vec<Arc<Ship>>,
    ships: Vec<Arc<Ship>>,
    user: User,
    btn_1_disabled: bool,
    btn_2_disabled: bool,
    btn_3_disabled: bool,
}
impl RouletteView {
    fn new(candidates: Vec<Ship>, players: RoulettePlayer, user: User) -> Self {
        let candidates: Vec<_> = candidates.into_iter().map(Arc::new).collect();
        RouletteView {
            ships: candidates
                .choose_multiple(&mut rand::rng(), players as usize)
                .cloned()
                .collect(),
            players,
            candidates,
            user,
            btn_1_disabled: false,
            btn_2_disabled: false,
            btn_3_disabled: false,
        }
    }
    fn reroll(&mut self, index: usize) -> &Self {
        self.ships[index] = self.candidates.choose(&mut rand::rng()).unwrap().clone();
        self
    }

    fn embed_build(&mut self) -> CreateEmbed {
        const EMOJI: [&str; 3] = ["1️⃣", "2️⃣", "3️⃣"];
        let author = CreateEmbedAuthor::new(&self.user.name)
            .icon_url(self.user.avatar_url().unwrap_or_default());

        let mut msg_text = String::new();
        for (index, ship) in self.ships.iter().enumerate() {
            msg_text += format!("{} {ship}\n\n", EMOJI[index]).as_str();
        }
        CreateEmbed::default_isac()
            .description(msg_text)
            .author(author)
    }
    async fn interactions(
        &mut self,
        ctx: &Context<'_>,
        author: UserId,
        mut msg: Message,
        duration: std::time::Duration,
    ) -> Result<(), Error> {
        while let Some(interaction) = msg
            .await_component_interactions(ctx)
            .timeout(duration)
            .author_id(author)
            .stream()
            .next()
            .await
        {
            match interaction.data.custom_id.as_str() {
                "roulette_1" => {
                    self.reroll(0);
                }
                "roulette_2" => {
                    self.reroll(1);
                }
                "roulette_3" => {
                    self.reroll(2);
                }
                _ => (),
            }
            interaction
                .create_response(
                    ctx,
                    CreateInteractionResponse::UpdateMessage(
                        CreateInteractionResponseMessage::default().embed(self.embed_build()),
                    ),
                )
                .await?;
        }
        // timeout;
        msg.edit(
            ctx,
            EditMessage::default().components(self.timeout().build()),
        )
        .await?;

        Ok(())
    }
    fn timeout(&mut self) -> &mut Self {
        self.btn_1_disabled = true;
        self.btn_2_disabled = true;
        self.btn_3_disabled = true;
        self
    }
    /// build the [`Vec<CreateActionRow>`] with current components state
    fn build(&self) -> Vec<CreateActionRow> {
        let mut btns = vec![
            CreateButton::new("roulette_1")
                .label("1️⃣🔄")
                .style(ButtonStyle::Secondary)
                .disabled(self.btn_1_disabled),
        ];
        if self.players as usize >= 2 {
            btns.push(
                CreateButton::new("roulette_2")
                    .label("2️⃣🔄")
                    .style(ButtonStyle::Secondary)
                    .disabled(self.btn_2_disabled),
            );
        }
        if self.players as usize >= 3 {
            btns.push(
                CreateButton::new("roulette_3")
                    .label("3️⃣🔄")
                    .style(ButtonStyle::Secondary)
                    .disabled(self.btn_3_disabled),
            );
        }
        vec![CreateActionRow::Buttons(btns)]
    }
}

#[derive(Debug, poise::ChoiceParameter, Clone, Copy)]
pub enum RoulettePlayer {
    #[name = "1"]
    One = 1,
    #[name = "2"]
    Two = 2,
    #[name = "3"]
    Three = 3,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, poise::ChoiceParameter, Clone, Copy)]
pub enum RouletteTier {
    I = 1,
    II = 2,
    III = 3,
    IV = 4,
    V = 5,
    VI = 6,
    VII = 7,
    VIII = 8,
    IX = 9,
    X = 10,
    XI = 11,
}

#[poise::command(prefix_command)]
pub async fn uid(ctx: Context<'_>, #[rest] mut args: Args) -> Result<(), Error> {
    let player = args.parse_user(&ctx).await?;
    let _r = ctx.reply(player.uid.to_string()).await;
    Ok(())
}

#[poise::command(prefix_command, aliases("clanid"))]
pub async fn clanuid(ctx: Context<'_>, #[rest] args: Option<Args>) -> Result<(), Error> {
    let mut args = args.unwrap_or_default();
    let first_arg = args.check(0)?;
    // parse region
    let region = match Region::parse(first_arg) {
        Some(region) => {
            args.remove(0)?;
            region
        }
        None => Region::guild_default(&ctx).await,
    };
    let clan_name = args.check(0)?;
    let clan = WowsApi::new(&ctx)
        .clans(&region, clan_name)
        .await?
        .swap_remove(0);
    let _r = ctx
        .reply(format!("`{}`'s UID: **{}**", clan, clan.id))
        .await;
    Ok(())
}
