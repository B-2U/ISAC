use std::{fmt::Write, str::FromStr, time::Duration};

use itertools::Itertools;
use once_cell::sync::Lazy;
use poise::{
    CreateReply,
    serenity_prelude::{
        ButtonStyle, CreateActionRow, CreateButton, CreateEmbed, CreateEmbedAuthor, Message,
        ReactionType, User, UserId,
    },
};
use regex::Regex;

use crate::{
    Context,
    structs::{Mode, PartialClan, PartialPlayer, Region, Ship},
    utils::{IsacError, IsacHelp, IsacInfo, wws_api::WowsApi},
};

use super::{EasyEmbed, UserAddon};

#[derive(Clone, Debug, Default)]
pub struct Args(Vec<String>);

impl Args {
    /// try to parse discord user at first, if none, parsing region and searching ign
    pub async fn parse_user(&mut self, ctx: &Context<'_>) -> Result<PartialPlayer, IsacError> {
        let first_arg = self.check(0)?;

        if let Ok(user) =
            User::convert_strict(ctx.serenity_context(), ctx.guild_id(), None, first_arg).await
        {
            let player = user.get_player(ctx).await?;
            self.remove(0)?;
            Ok(player)
        } else if first_arg == "me" {
            let player = ctx.author().get_player(ctx).await?;
            self.remove(0)?;
            Ok(player)
        } else {
            // parse region, player
            let region = self.parse_region(ctx).await?;
            let ign = self.check(0)?;

            let api = WowsApi::new(ctx);
            let candidates = api.players(&region, ign, 4).await?;
            let player = match candidates.len() {
                0 => Err(IsacInfo::PlayerIgnNotFound {
                    ign: ign.to_string(),
                    region,
                })?,
                1 => {
                    self.remove(0)?;
                    &candidates[0]
                }
                _ => {
                    let index = self._pick(ctx, &candidates).await?;
                    self.remove(0)?;
                    &candidates[index]
                }
            };
            Ok(PartialPlayer {
                region,
                uid: player.uid,
            })
        }
    }

    /// try to parse discord user at first, if none, parsing region and searching ign
    pub async fn parse_clan(&mut self, ctx: &Context<'_>) -> Result<PartialClan, IsacError> {
        let first_arg = self.check(0)?;
        let api = WowsApi::new(ctx);

        if let Ok(user) =
            User::convert_strict(ctx.serenity_context(), ctx.guild_id(), None, first_arg).await
        {
            let linked_user = user.get_player(ctx).await?;
            self.remove(0)?;
            linked_user.clan(&api).await
        } else if first_arg == "me" {
            let linked_user = ctx.author().get_player(ctx).await?;
            self.remove(0)?;
            linked_user.clan(&api).await
        } else {
            // parse region, clan
            let region = self.parse_region(ctx).await?;
            let clan_name = self.remove(0)?;

            let mut clans = api.clans(&region, &clan_name).await?;
            Ok(clans.remove(0))
        }
    }

    /// parsing region, return guild default or Asia if not specific
    pub async fn parse_region(&mut self, ctx: &Context<'_>) -> Result<Region, IsacError> {
        let first_arg = self.check(0)?;
        Ok(match Region::parse(first_arg) {
            Some(region) => {
                self.remove(0)?;
                region
            }
            None => Region::guild_default(ctx).await,
        })
    }

    /// parsing battle modes, return None if there is only no matched
    pub fn parse_mode(&mut self) -> Option<Mode> {
        if let Some(index) = self.0.iter().rposition(|key| Mode::parse(key).is_some()) {
            Mode::parse(&self.remove(index).unwrap())
        } else {
            None
        }
    }

    /// looking for number between 1 - 100, for `recent` days
    pub fn parse_day(&mut self) -> Option<u64> {
        if let Some(index) = self.0.iter().rposition(|key| key.parse::<u32>().is_ok()) {
            self.remove(index).unwrap().parse::<u64>().ok()
        } else {
            None
        }
    }

    /// searching for matching ships' name, Err if no argument left || No matched ship || Error when user picking
    ///
    /// ## Note:
    /// this should be runned in the last, since it will consume all the remained arguments
    pub async fn parse_ship(&mut self, ctx: &Context<'_>) -> Result<Ship, IsacError> {
        self.check(0)?;
        // removed all the arguments left, it might need to be changed if there's new needed arguments added in the future
        let ship_input = self.0.iter().join(" ");
        self.0.clear();

        let mut candidates = {
            let ship_js = ctx.data().ships.read();
            ship_js.search_name(&ship_input, 4)?
        };

        let index = match candidates.len() == 1 {
            true => 0,
            false => self._pick(ctx, &candidates).await?,
        };
        Ok(candidates.remove(index))
    }
    /// let user select the ship or player from candidates
    async fn _pick<T: std::fmt::Display>(
        &self,
        ctx: &Context<'_>,
        candidates: &[T],
    ) -> Result<usize, IsacError> {
        let view = PickView::new(candidates, ctx.author());
        let inter_msg = ctx
            .send(
                CreateReply::default()
                    .components(view.build())
                    .embed(view.embed_build()),
            )
            .await
            .map_err(|_| IsacInfo::EmbedPermission)?
            .into_message()
            .await
            .map_err(|_| IsacError::Cancelled)?;
        match view.interactions(ctx, ctx.author().id, inter_msg).await {
            Some(index) => Ok(index as usize),
            None => Err(IsacError::Cancelled)?,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// remove the given index in args safely, raise [`IsacError::LackOfArguments`] if it is out of index
    pub fn remove(&mut self, index: usize) -> Result<String, IsacError> {
        match index < self.0.len() {
            true => Ok(self.0.remove(index)),
            false => Err(IsacError::Help(IsacHelp::LackOfArguments)),
        }
    }
    /// check if the index is in args safely, raise [`IsacError::LackOfArguments`] if it is out of index
    pub fn check(&self, index: isize) -> Result<&str, IsacError> {
        let index = if index.is_negative() {
            let index = -index as usize;
            if index <= self.0.len() {
                self.0.len() - index
            } else {
                return Err(IsacError::Help(IsacHelp::LackOfArguments));
            }
        } else {
            index as usize
        };
        match self.0.get(index) {
            Some(player_id) => Ok(player_id),
            None => Err(IsacError::Help(IsacHelp::LackOfArguments))?,
        }
    }
}

impl FromStr for Args {
    type Err = IsacError;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#""([^"]+)"|([^\s"]+)"#).unwrap());
        Ok(Args(
            RE.captures_iter(input)
                .map(|c| {
                    c.get(1)
                        .or_else(|| c.get(2))
                        .expect("it's either 1 or 2")
                        .as_str()
                        .trim()
                        .to_string()
                })
                .filter(|s| !s.is_empty())
                .collect(),
        ))
    }
}
impl From<Args> for Vec<String> {
    fn from(value: Args) -> Self {
        value.0
    }
}

struct PickView<'a, T> {
    candidates: &'a [T],
    user: &'a User,
    emoji: [&'static str; 4],
    x_emoji: &'static str,
}

impl<'a, T: std::fmt::Display> PickView<'a, T> {
    fn new(candidates: &'a [T], user: &'a User) -> Self {
        Self {
            candidates,
            user,
            emoji: ["1️⃣", "2️⃣", "3️⃣", "4️⃣"],
            x_emoji: "❌",
        }
    }

    fn embed_build(&self) -> CreateEmbed {
        let mut msg_text = self.candidates.iter().enumerate().fold(
            String::new(),
            |mut buf, (index, candidate)| {
                let _ = writeln!(buf, "{} {}\n", self.emoji[index], candidate);
                buf
            },
        );

        msg_text += format!("{} None of above", self.x_emoji).as_str();

        CreateEmbed::default_isac()
            .author(
                CreateEmbedAuthor::new(&self.user.name)
                    .icon_url(self.user.avatar_url().unwrap_or_default()),
            )
            .description(msg_text)
    }
    async fn interactions(&self, ctx: &Context<'_>, author: UserId, msg: Message) -> Option<u8> {
        let result = match msg
            .await_component_interaction(ctx)
            .timeout(Duration::from_secs(15))
            .author_id(author)
            .await
        {
            Some(interaction) => match interaction.data.custom_id.as_str() {
                "select_1" => Some(0),
                "select_2" => Some(1),
                "select_3" => Some(2),
                "select_4" => Some(3),
                "select_x" => None,
                _ => None,
            },
            None => None,
        };
        let _ok = msg.delete(ctx).await;
        result
    }

    /// build the `CreateActionRow` with current components state
    fn build(&self) -> Vec<CreateActionRow> {
        let mut btns = (1..=self.candidates.len())
            .map(|i| {
                CreateButton::new(format!("select_{}", i))
                    .emoji(ReactionType::Unicode(self.emoji[i - 1].to_string()))
                    .style(ButtonStyle::Secondary)
            })
            .collect::<Vec<_>>();

        let btn_x = CreateButton::new("select_x")
            .emoji(ReactionType::Unicode(self.x_emoji.to_string()))
            .style(ButtonStyle::Secondary);
        btns.push(btn_x);

        vec![CreateActionRow::Buttons(btns)]
    }
}
