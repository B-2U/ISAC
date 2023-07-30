use std::collections::HashMap;

use once_cell::sync::Lazy;
use poise::{
    futures_util::StreamExt,
    serenity_prelude::{
        ButtonStyle, CreateActionRow, CreateButton, CreateEmbed, CreateEmbedAuthor, Message, User,
        UserId,
    },
};
use rand::seq::SliceRandom;

use crate::{
    dc_utils::{Args, ContextAddon, EasyEmbed, InteractionAddon},
    utils::{LoadFromJson, Ship, ShipsPara},
    Context, Error,
};

#[poise::command(prefix_command)]
pub async fn rename(ctx: Context<'_>, #[rest] args: Option<Args>) -> Result<(), Error> {
    let mut args = args.unwrap_or_default();
    let temp = args.parse_user(&ctx).await?.get_player(&ctx).await?;
    ctx.reply(format!("{:?}", temp)).await?;
    // let args: Vec<_> = args.unwrap_or_default().into();
    // ctx.reply(format!("{}", args.unwrap_or_default())).await?;
    Ok(())
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
        .iter()
        .filter(|(_ship_id, ship)| ship.tier == tier.to_int::<u32>() && ship.is_available())
        .map(|(_ship_id, ship)| ship.clone())
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
// todo: ships are actually borrow of candidates, but lifetimes....
struct RouletteView {
    players: RoulettePlayer,
    candidates: Vec<Ship>,
    ships: Vec<Ship>,
    user: User,
    btn_1: CreateButton,
    btn_2: CreateButton,
    btn_3: CreateButton,
}
impl RouletteView {
    fn new(candidates: Vec<Ship>, players: RoulettePlayer, user: User) -> Self {
        let btn_style = ButtonStyle::Secondary;
        RouletteView {
            ships: candidates
                .choose_multiple(&mut rand::thread_rng(), players.to_int())
                .map(|m| m.clone())
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
        if self.players.to_int::<u8>() >= 2 {
            row.add_button(self.btn_2.clone());
        }
        if self.players.to_int::<u8>() >= 3 {
            row.add_button(self.btn_3.clone());
        }
        row.to_owned()
    }
}

#[derive(Debug, poise::ChoiceParameter, Clone)]
pub enum RoulettePlayer {
    #[name = "1"]
    One,
    #[name = "2"]
    Two,
    #[name = "3"]
    Three,
}
// todo: better to_int() way here? (poise require enum)
impl RoulettePlayer {
    fn to_int<T>(&self) -> T
    where
        T: From<u8>,
    {
        match self {
            Self::One => T::from(1_u8),
            Self::Two => T::from(2_u8),
            Self::Three => T::from(3_u8),
        }
    }
}

#[derive(Debug, poise::ChoiceParameter)]
pub enum RouletteTier {
    I,
    II,
    III,
    IV,
    V,
    VI,
    VII,
    VIII,
    IX,
    X,
    XI,
}
// todo: better local static map/array? (https://github.com/rust-phf/rust-phf ?)
impl RouletteTier {
    fn to_int<T>(&self) -> T
    where
        T: From<u8>,
    {
        static TIER_ROMAN_TO_NUM_MAP: Lazy<HashMap<&'static str, u8>> = Lazy::new(|| {
            HashMap::from([
                ("I", 1),
                ("II", 2),
                ("III", 3),
                ("IV", 4),
                ("V", 5),
                ("VI", 6),
                ("VII", 7),
                ("VIII", 8),
                ("IX", 9),
                ("X", 10),
                ("XI", 11),
            ])
        });

        T::from(*TIER_ROMAN_TO_NUM_MAP.get(self.name()).unwrap())
    }
}
