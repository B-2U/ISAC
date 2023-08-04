use std::{borrow::Cow, sync::Arc};

use chrono::NaiveDate;
use poise::{
    futures_util::StreamExt,
    serenity_prelude::{
        AttachmentType, ButtonStyle, CreateActionRow, CreateButton, CreateEmbed, CreateEmbedAuthor,
        Message, ReactionType, User, UserId,
    },
    FrameworkError,
};
use rand::seq::SliceRandom;
use regex::Regex;
use scraper::{Element, Html, Selector};

use crate::{
    dc_utils::{Args, ContextAddon, EasyEmbed, InteractionAddon},
    utils::{structs::PartialPlayer, IsacError, LoadFromJson, PlayerCommon, Ship, ShipsPara},
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
                        .components(|c| c.set_action_row(view.build()))
                        .reply(true)
                })
                .await?
                .into_message()
                .await?
            }
            _ => {
                ctx.channel_id()
                    .send_message(&ctx, |b| {
                        b.content(format!("wows code: **{code}**"))
                            .components(|c| c.set_action_row(view.build()))
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
    fn build(&self) -> CreateActionRow {
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
        let mut row = CreateActionRow::default();
        LABEL.iter().zip(buttons.iter()).for_each(|(label, btn)| {
            row.create_button(|b| b.label(label).url(btn).style(ButtonStyle::Link));
        });
        row
    }
}

#[poise::command(prefix_command)]
pub async fn rename(ctx: Context<'_>, #[rest] args: Option<Args>) -> Result<(), Error> {
    let mut args = args.unwrap_or_default();
    let player = args.parse_user(&ctx).await?;
    let _typing = ctx.typing().await;
    let res = ctx
        .data()
        .client
        .get(player.wows_number()?)
        .send()
        .await
        .map_err(|err| IsacError::UnknownError(Box::new(err)))?;
    let text = res.text().await.unwrap();
    let record_clans_uid =
        _rename_parse_player(text).map_err(|err| IsacError::UnknownError(err))?;

    // todo: since theres async function inside, guess i can't use map() or flat_map() to replace for loop?
    let mut name_history = vec![];
    for clan_uid in record_clans_uid {
        // todo: is boxing error a good practice?
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
        name_history
            .extend(_rename_parse_clan(text, &player).map_err(|err| IsacError::UnknownError(err))?);
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
        .map(|(date, ign)| format!("{}, {}\n", ign, date.format("%d.%m.%Y")))
        .collect::<String>();
    let output = format!("```py\n{}\n```", output);
    if let Err(_) = ctx.reply(&output).await {
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

fn _rename_parse_player(html_text: impl AsRef<str>) -> Result<Vec<u64>, Error> {
    let html = Html::parse_document(html_text.as_ref());

    let table_selector = Selector::parse(".table-styled").unwrap();
    let transfer_s_selector = Selector::parse(".col.col-centered.col-sm-6").unwrap();

    // should be only 2 here,[ Important moments, Transfer ]
    let transfer_s = html.select(&transfer_s_selector).nth(1).unwrap();
    let tables = transfer_s.select(&table_selector).collect::<Vec<_>>();
    let target_table = match tables.len() {
        1 => tables[0],
        2 => tables[1],
        _ => Err("target_table.len() is not 1 or 2")?,
    };
    let cells_selector = Selector::parse("tr").unwrap();
    let a_selector = Selector::parse("a").unwrap();
    let cells = target_table.select(&cells_selector);

    let clan_regex = Regex::new(r"/clan/(\d+),").unwrap();
    Ok(cells
        .filter_map(|cell| {
            let _clan_href = cell
                .select(&a_selector)
                .nth(0)
                .and_then(|f| f.value().attr("href"))?; //todo: why i can "?" a Option here?
            clan_regex
                .captures(_clan_href)?
                .get(1)
                .and_then(|uid| uid.as_str().parse::<u64>().ok())
        })
        .collect())
}

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

#[poise::command(slash_command, discard_spare_arguments)]
pub async fn roulette(
    ctx: Context<'_>,
    #[description = "how many players in the div? default: 3"] players: Option<RoulettePlayer>,
    #[description = "ships tier, default: 10"] tier: Option<RouletteTier>,
) -> Result<(), Error> {
    let players = players.unwrap_or(RoulettePlayer::Three);
    let tier = tier.unwrap_or(RouletteTier::X);
    let ship_js = ShipsPara::load_json("./web_src/ship/ships_para.json").await?;
    let cadidates = ship_js
        .0
        .into_iter()
        .filter(|(_ship_id, ship)| ship.tier == tier as u8 && ship.is_available())
        .map(|(_ship_id, ship)| ship)
        .collect::<Vec<_>>();
    // let mut ships: Vec<Ship> = cadidates
    //     .choose_multiple(&mut rand::thread_rng(), players.to_int())
    //     .map(|&m| m.clone())
    //     .collect();

    let mut view: RouletteView = RouletteView::new(cadidates, players, ctx.author().clone());

    let embed = view.embed_build();
    let inter_msg = ctx
        .send(|b| {
            b.embeds = vec![embed];
            b.components(|f| f.set_action_row(view.build()))
        })
        .await?
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
        let embed = CreateEmbed::default_new()
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
            .await_component_interactions(&ctx)
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
        msg.edit(ctx, |m| {
            m.components(|f| f.add_action_row(self.timeout().build()))
        })
        .await?;

        Ok(())
    }
    fn timeout(&mut self) -> &mut Self {
        self.btn_1.disabled(true);
        self.btn_2.disabled(true);
        self.btn_3.disabled(true);
        self
    }
    /// build the `CreateActionRow` with current components state
    fn build(&self) -> CreateActionRow {
        let mut row = CreateActionRow::default();
        row.add_button(self.btn_1.clone());
        if self.players as usize >= 2 {
            row.add_button(self.btn_2.clone());
        }
        if self.players as usize >= 3 {
            row.add_button(self.btn_3.clone());
        }
        row.to_owned()
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
pub async fn uid(ctx: Context<'_>, #[rest] args: Option<Args>) -> Result<(), Error> {
    let mut args = args.unwrap_or_default();
    let player = args.parse_user(&ctx).await?.get_player(&ctx).await?;
    let _r = ctx
        .reply(format!("`{}`'s UID: **{}**", player.ign, player.uid))
        .await;
    Ok(())
}

#[poise::command(prefix_command, aliases("clanid"))]
pub async fn clanuid(ctx: Context<'_>, #[rest] args: Option<Args>) -> Result<(), Error> {
    let mut args = args.unwrap_or_default();
    let clan = args.parse_clan(&ctx).await?;
    let _r = ctx
        .reply(format!("`{}`'s UID: **{}**", clan.tag, clan.id))
        .await;
    Ok(())
}
