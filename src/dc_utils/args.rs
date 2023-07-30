use std::{collections::HashMap, error::Error, str::FromStr, time::Duration};

use poise::serenity_prelude::{
    ButtonStyle, CreateActionRow, CreateButton, CreateEmbed, CreateEmbedAuthor, Message,
    ReactionType, User, UserId,
};

use crate::{
    utils::{
        user::{Linked, PartialPlayer, Region},
        wws_api::{VortexPlayer, WowsApi},
        IsacError, IsacHelp, IsacInfo,
    },
    Context,
};

use super::{EasyEmbed, UserAddon};

#[derive(Clone, Debug)]
pub struct Args(Vec<String>);

// todo: improve the args removing logic
impl Args {
    pub async fn parse_user(&mut self, ctx: &Context<'_>) -> Result<PartialPlayer, IsacError> {
        let linked_js: HashMap<_, _> = Linked::load().await.into();

        let first_arg = self.check(0)?;

        if let Ok(user) =
            User::convert_strict(ctx.serenity_context(), ctx.guild_id(), None, first_arg).await
        {
            match linked_js.get(&user.id) {
                Some(linked_user) => {
                    self.remove(0)?;
                    Ok(*linked_user)
                }
                None => {
                    return Err(IsacError::Info(IsacInfo::UserNotLinked {
                        msg: format!("**{}** haven't linked to any wows account yet", user.name),
                    }));
                }
            }
        } else if first_arg == "me" {
            match linked_js.get(&ctx.author().id) {
                Some(linked_user) => {
                    self.remove(0)?;
                    Ok(*linked_user)
                }
                None => {
                    return Err(IsacError::Info(IsacInfo::UserNotLinked {
                        msg: format!("You haven't linked your account yet.\nEnter `/link`"),
                    }));
                }
            }
        } else {
            // parse region, player
            let region = match Region::parse(first_arg) {
                Some(region) => {
                    self.remove(0)?;
                    region
                }
                None => Region::guild_default(ctx.guild_id()).await,
            };
            let player_id = self.check(0)?;

            let api = WowsApi(&ctx.data().client);
            let candidates = match api.players(&region, player_id, 4).await {
                Ok(result) => result,
                Err(err) => Err(err)?,
            };
            let player = match candidates.len() {
                0 => Err(IsacError::Info(IsacInfo::PlayerIgnNotFound {
                    ign: player_id.to_string(),
                    region,
                }))?,
                1 => {
                    self.remove(0)?;
                    &candidates[0]
                }
                _ => {
                    let Ok(index) = Self::_pick_player(ctx, ctx.author(), &candidates).await else {
                        Err(IsacError::Cancelled)?
                    };
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
    async fn _pick_player(
        ctx: &Context<'_>,
        author: &User,
        players: &Vec<VortexPlayer>,
    ) -> Result<usize, Box<dyn Error + Send + Sync>> {
        let view = PickView::new(&players, author);
        let embed = view.embed_build();
        let inter_msg = ctx
            .send(|b| {
                b.embeds = vec![embed];
                b.components(|f| f.set_action_row(view.build()))
            })
            .await?
            .into_message()
            .await?;
        match view.interactions(&ctx, ctx.author().id, inter_msg).await {
            Some(index) => {
                return Ok(index as usize);
            }
            None => Err(IsacError::Cancelled)?,
        }
    }
    /// remove the given index in args safely, raise [`IsacError::LackOfArguments`] if it is out of index
    fn remove(&mut self, index: usize) -> Result<(), IsacError> {
        match index < self.0.len() {
            true => {
                self.0.remove(index);
                Ok(())
            }
            false => Err(IsacError::Help(IsacHelp::LackOfArguments)),
        }
    }
    /// check if the index is in args safely, raise [`IsacError::LackOfArguments`] if it is out of index
    fn check(&self, index: usize) -> Result<&str, IsacError> {
        match self.0.get(index) {
            Some(player_id) => Ok(player_id),
            None => Err(IsacError::Help(IsacHelp::LackOfArguments))?,
        }
    }
}

impl FromStr for Args {
    type Err = IsacError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Args(s.split_whitespace().map(|f| f.to_string()).collect()))
    }
}
impl From<Args> for Vec<String> {
    fn from(value: Args) -> Self {
        value.0
    }
}
impl Default for Args {
    fn default() -> Self {
        Self(Default::default())
    }
}

struct PickView<'a, T> {
    candidates: &'a Vec<T>,
    user: &'a User,
    emoji: [&'static str; 4],
    x_emoji: &'static str,
}

impl<'a, T: std::fmt::Display> PickView<'a, T> {
    fn new(candidates: &'a Vec<T>, user: &'a User) -> Self {
        Self {
            candidates,
            user,
            emoji: ["1️⃣", "2️⃣", "3️⃣", "4️⃣"],
            x_emoji: "❌",
        }
    }

    fn embed_build(&self) -> CreateEmbed {
        let author = CreateEmbedAuthor::default()
            .name(&self.user.name)
            .icon_url(self.user.avatar_url().unwrap_or_default())
            .to_owned();

        let mut msg_text = String::new();
        for (index, candidate) in self.candidates.iter().enumerate() {
            msg_text += format!("{} {candidate}\n\n", self.emoji[index]).as_str();
        }
        msg_text += format!("{} None of above", self.x_emoji).as_str();
        let embed = CreateEmbed::default_new()
            .description(msg_text)
            .set_author(author)
            .to_owned();
        embed
    }
    async fn interactions(&self, ctx: &Context<'_>, author: UserId, msg: Message) -> Option<u8> {
        let result = match msg
            .await_component_interaction(&ctx)
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
    fn build(&self) -> CreateActionRow {
        const BTN_STYLE: ButtonStyle = ButtonStyle::Secondary;
        let mut row = CreateActionRow::default();
        if self.candidates.len() >= 1 {
            let btn_1 = CreateButton::default()
                .emoji(ReactionType::Unicode(self.emoji[0].to_string()))
                .custom_id("select_1")
                .style(BTN_STYLE)
                .to_owned();
            row.add_button(btn_1);
        }
        if self.candidates.len() >= 2 {
            let btn_2 = CreateButton::default()
                .emoji(ReactionType::Unicode(self.emoji[1].to_string()))
                .custom_id("select_2")
                .style(BTN_STYLE)
                .to_owned();
            row.add_button(btn_2);
        }
        if self.candidates.len() >= 3 {
            let btn_3 = CreateButton::default()
                .emoji(ReactionType::Unicode(self.emoji[2].to_string()))
                .custom_id("select_3")
                .style(BTN_STYLE)
                .to_owned();
            row.add_button(btn_3);
        }
        if self.candidates.len() >= 4 {
            let btn_4 = CreateButton::default()
                .emoji(ReactionType::Unicode(self.emoji[3].to_string()))
                .custom_id("select_4")
                .style(BTN_STYLE)
                .to_owned();
            row.add_button(btn_4);
        }
        let btn_x = CreateButton::default()
            .emoji(ReactionType::Unicode(self.x_emoji.to_string()))
            .custom_id("select_x")
            .style(BTN_STYLE)
            .to_owned();
        row.add_button(btn_x);
        row.to_owned()
    }
}
