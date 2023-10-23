use std::{borrow::Cow, fmt::Write, sync::Arc};

use chrono::NaiveDate;
use poise::{
    futures_util::StreamExt,
    serenity_prelude::{
        AttachmentType, ButtonStyle, CreateActionRow, CreateButton, CreateComponents, CreateEmbed,
        CreateEmbedAuthor, Message, ReactionType, User, UserId,
    },
    FrameworkError,
};
use rand::seq::SliceRandom;
use regex::Regex;
use scraper::{Element, Html, Selector};

use crate::{
    dc_utils::{Args, ContextAddon, CreateReplyAddon, EasyEmbed, InteractionAddon},
    utils::{
        structs::{PartialPlayer, Region, Ship},
        wws_api::WowsApi,
        IsacError, IsacInfo,
    },
    Context, Data, Error,
};

/// The link to wargaming wiki maps page
#[poise::command(prefix_command, slash_command, discard_spare_arguments)]
pub async fn map(ctx: Context<'_>) -> Result<(), Error> {
    let _r = ctx.reply("https://wiki.wargaming.net/en/Ship:Maps").await;
    Ok(())
}

async fn code_err_handler(err: FrameworkError<'_, Data, Error>) {
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
    let codes_vec = codes.split_whitespace().collect::<Vec<_>>();
    // if there's more than one code, adding a code block for user to copy
    let addition = match codes_vec.len() {
        1 => String::new(),
        _ => format!("```.code {codes}```\n"),
    };
    for (index, code) in codes_vec.iter().enumerate() {
        let view = BonusView::new(code);
        let last_msg = match index {
            0 => {
                ctx.send(|b| {
                    b.content(format!("{addition}wows code: **{code}**"))
                        .set_components(view.build())
                        .reply(true)
                })
                .await?
                .into_message()
                .await?
            }
            _ => {
                ctx.channel_id()
                    .send_message(&ctx, |b| {
                        b.set_components(view.build())
                            .content(format!("wows code: **{code}**"))
                    })
                    .await?
            }
        };
        if index == codes_vec.len() - 1 {
            let _r = last_msg
                .react(ctx, ReactionType::Unicode("❤️".to_string()))
                .await;
        }
    }
    Ok(())
}

struct BonusView<'a> {
    code: &'a str,
}

impl<'a> BonusView<'a> {
    fn new(code: &'a str) -> Self {
        Self { code }
    }
    fn build(&self) -> CreateComponents {
        // sepereated them to 2 array becuz with a Array<Tuple> it formatted ugly
        const LABEL: [&str; 4] = ["Asia", "Na", "Eu", "Ru"];
        let buttons = [
            format!(
                "https://asia.wargaming.net/shop/bonus/?bonus_mode={}",
                self.code
            ),
            format!(
                "https://na.wargaming.net/shop/bonus/?bonus_mode={}",
                self.code
            ),
            format!(
                "https://eu.wargaming.net/shop/bonus/?bonus_mode={}",
                self.code
            ),
            format!("https://lesta.ru/shop/bonus/?bonus_mode={}", self.code),
        ];
        let mut view = CreateComponents::default();
        let mut row = CreateActionRow::default();
        LABEL.iter().zip(buttons.iter()).for_each(|(label, btn)| {
            row.create_button(|b| b.label(label).url(btn).style(ButtonStyle::Link));
        });
        view.set_action_row(row);
        view
    }
}

#[poise::command(prefix_command, discard_spare_arguments)]
pub async fn rename(ctx: Context<'_>) -> Result<(), Error> {
    let _r = ctx.reply("`rename` is called `history now`").await;
    Ok(())
}

#[poise::command(prefix_command)]
pub async fn history(ctx: Context<'_>, #[rest] mut args: Args) -> Result<(), Error> {
    let _typing = ctx.typing().await;
    let api = WowsApi::new(&ctx);
    let player = args.parse_user(&ctx).await?;
    // this is just for rasing error when player profile is hidden
    let _ = player.get_player(&api).await?;
    let res_text = ctx
        .data()
        .client
        .get(player.wows_number_url()?)
        .send()
        .await?
        .text()
        .await?;
    let record_clans_uid = _rename_parse_player(res_text)?;

    let mut name_history = vec![];
    for (clan_uid, clan_tag) in record_clans_uid {
        let res = ctx
            .data()
            .client
            .get(
                player
                    .region
                    .number_url(format!("/clan/transfers/{clan_uid},/"))?,
            )
            .send()
            .await
            .map_err(|err| IsacError::UnknownError(Box::new(err)))?;
        let text = res.text().await.unwrap();
        let transfers = _rename_parse_clan(text, &player).map_err(IsacError::UnknownError)?;
        name_history.extend(
            transfers
                .into_iter()
                .map(|(date, ign)| (date, ign, clan_tag.clone())),
        );
    }
    name_history.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    let filtered_history: Vec<_> = name_history
        .iter()
        .enumerate()
        .filter(|(index, record)| {
            if let (Some(left), Some(right)) = (
                name_history.get(index.wrapping_sub(1)),
                name_history.get(index + 1),
            ) {
                record.1 != left.1 && record.1 != right.1
            } else {
                true
            }
        })
        .map(|(_, history)| history)
        .collect();

    let output = filtered_history
        .iter()
        .fold(String::new(), |mut buf, (date, ign, clan_tag)| {
            let _ = writeln!(buf, "{ign} {clan_tag}, {}", date.format("%d.%m.%Y"));
            buf
        });
    let output = format!("```py\n{}\n```", output);
    if ctx.reply(&output).await.is_err() {
        // the history might exceed the limit
        let _r = ctx
            .send(|b| {
                b.attachment(AttachmentType::Bytes {
                    data: Cow::Borrowed(output.as_bytes()),
                    filename: "rename history".to_string(),
                })
            })
            .await;
    }
    Ok(())
}

/// return clan's uid and tag, Vec<(uid, tag)>
fn _rename_parse_player(html_text: impl AsRef<str>) -> Result<Vec<(u64, String)>, IsacError> {
    let html = Html::parse_document(html_text.as_ref());

    let table_selector = Selector::parse(".table-styled").unwrap();
    let transfer_s_selector = Selector::parse(".col.col-centered.col-sm-6").unwrap();

    // should be only 2 here,[ Important moments, Transfer ]
    let transfer_s = html.select(&transfer_s_selector).nth(1).unwrap();
    let tables = transfer_s.select(&table_selector).collect::<Vec<_>>();
    let target_table = match tables.len() {
        1 => tables[0],
        2 => tables[1],
        _ => Err(IsacInfo::GeneralError {
            msg: "No transfer history".to_string(),
        })?,
    };
    let cells_selector = Selector::parse("tr").unwrap();
    let a_selector = Selector::parse("a").unwrap();
    let cells = target_table.select(&cells_selector);

    let clan_uid_regex = Regex::new(r"/clan/(\d+),").unwrap();
    Ok(cells
        .filter_map(|cell| {
            let selected = cell.select(&a_selector).next()?;
            let clan_name = selected
                .inner_html()
                .split_whitespace()
                .next()
                .unwrap_or("[]")
                .to_string();
            let _clan_href = selected.value().attr("href")?;
            clan_uid_regex
                .captures(_clan_href)?
                .get(1)
                .and_then(|uid| uid.as_str().parse::<u64>().ok())
                .map(|i| (i, clan_name))
        })
        .collect())
}

/// parsing the clan transfer html, return Vec<(NaiveDate, ign)>
fn _rename_parse_clan(
    html_text: impl AsRef<str>,
    player: &PartialPlayer,
) -> Result<Vec<(NaiveDate, String)>, Error> {
    let html = Html::parse_document(html_text.as_ref());

    let cells_selector = Selector::parse("table tr").unwrap();
    let cells = html.select(&cells_selector);

    let player_regex = Regex::new(r"/player/([^,]+),").unwrap();

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
        let player_uid = player_regex
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
        let ship_js = ctx.data().ship_js.read();
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
        .send(|b| b.set_embed(view.embed_build()).set_components(view.build()))
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
    btn_1: CreateButton,
    btn_2: CreateButton,
    btn_3: CreateButton,
}
impl RouletteView {
    fn new(candidates: Vec<Ship>, players: RoulettePlayer, user: User) -> Self {
        let btn_style = ButtonStyle::Secondary;
        let candidates: Vec<_> = candidates.into_iter().map(Arc::new).collect();
        RouletteView {
            ships: candidates
                .choose_multiple(&mut rand::thread_rng(), players as usize)
                .cloned()
                .collect(),
            players,
            candidates,
            user,
            btn_1: CreateButton::default()
                .label("1️⃣🔄")
                .custom_id("roulette_1")
                .style(btn_style)
                .to_owned(),
            btn_2: CreateButton::default()
                .label("2️⃣🔄")
                .custom_id("roulette_2")
                .style(btn_style)
                .to_owned(),
            btn_3: CreateButton::default()
                .label("3️⃣🔄")
                .custom_id("roulette_3")
                .style(btn_style)
                .to_owned(),
        }
    }
    fn reroll(&mut self, index: usize) -> &Self {
        self.ships[index] = self
            .candidates
            .choose(&mut rand::thread_rng())
            .unwrap()
            .clone();
        self
    }

    fn embed_build(&mut self) -> CreateEmbed {
        const EMOJI: [&str; 3] = ["1️⃣", "2️⃣", "3️⃣"];
        let author = CreateEmbedAuthor::default()
            .name(&self.user.name)
            .icon_url(self.user.avatar_url().unwrap_or_default())
            .to_owned();

        let mut msg_text = String::new();
        for (index, ship) in self.ships.iter().enumerate() {
            msg_text += format!("{} {ship}\n\n", EMOJI[index]).as_str();
        }
        let embed = CreateEmbed::isac()
            .description(msg_text)
            .set_author(author)
            .to_owned();
        embed
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
            .build()
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
                .edit_original_message(ctx, |f| f.set_embed(self.embed_build()))
                .await?;
        }
        // timeout;
        msg.edit(ctx, |m| m.set_components(self.timeout().build()))
            .await?;

        Ok(())
    }
    fn timeout(&mut self) -> &mut Self {
        self.btn_1.disabled(true);
        self.btn_2.disabled(true);
        self.btn_3.disabled(true);
        self
    }
    /// build the [`CreateComponents`] with current components state
    fn build(&self) -> CreateComponents {
        let mut view = CreateComponents::default();
        let mut row = CreateActionRow::default();
        row.add_button(self.btn_1.clone());
        if self.players as usize >= 2 {
            row.add_button(self.btn_2.clone());
        }
        if self.players as usize >= 3 {
            row.add_button(self.btn_3.clone());
        }
        view.set_action_row(row);
        view
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
    let api = WowsApi::new(&ctx);
    let player = args.parse_user(&ctx).await?.get_player(&api).await?;
    let _r = ctx
        .reply(format!("`{}`'s UID: **{}**", player.ign, player.uid))
        .await;
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
        .reply(format!("`{}`'s UID: **{}**", clan.tag, clan.id))
        .await;
    Ok(())
}
