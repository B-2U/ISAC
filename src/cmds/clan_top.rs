use std::time::Duration;

use chrono::{DateTime, Utc};
use futures::StreamExt;
use poise::{
    serenity_prelude::{
        ButtonStyle, CreateActionRow, CreateButton, CreateEmbed, CreateInteractionResponse,
        CreateInteractionResponseMessage, EditMessage, Message, ReactionType, UserId,
    },
    CreateReply,
};
use serde::Deserialize;

use crate::{
    structs::{ClanTag, Region},
    utils::IsacError,
    Context, Error,
};

/// The Clan Battle leaderboard
#[poise::command(slash_command)]
pub async fn clan_top(
    ctx: Context<'_>,
    #[max = 100]
    #[min = -1]
    #[description = "Clan Battle season, default: latest"]
    season: Option<i64>,
    #[description = "specific region, default: depend on server's default"] region: Option<Region>,
) -> Result<(), Error> {
    let region = match region {
        Some(region) => region,
        None => ctx
            .data()
            .guild_default
            .read()
            .await
            .get_default(ctx.guild_id()),
    };
    let season = {
        let season = season.unwrap_or(-1);
        match season.is_positive() {
            false => ctx.data().constant.read().clan_season,
            true => season as u32,
        }
    };
    let mut view = ClanTopView::new(ctx.data().client.clone(), region, season);
    let first_embed = view.build_embed().await?;
    let msg = ctx
        .send(
            CreateReply::default()
                .embed(first_embed)
                .components(view.build()),
        )
        .await?
        .into_message()
        .await?;
    view.interactions(&ctx, ctx.author().id, msg).await?;
    Ok(())
}

pub struct ClanTopView {
    client: reqwest::Client,
    ranks: &'static [[usize; 2]; 13],
    ranks_index: usize,
    region: Region,
    season: u32,
    timeout: bool,
}
impl ClanTopView {
    pub fn new(client: reqwest::Client, region: Region, season: u32) -> Self {
        Self {
            client,
            ranks: &[
                [4, 3], // squall 3
                [4, 2],
                [4, 1],
                [3, 3],
                [3, 2],
                [3, 1],
                [2, 3],
                [2, 2],
                [2, 1],
                [1, 3],
                [1, 2],
                [1, 1],
                [0, 1], // hurricane 1
            ],
            ranks_index: 12,
            region,
            season,
            timeout: false,
        }
    }

    async fn interactions(
        &mut self,
        ctx: &Context<'_>,
        author: UserId,
        mut msg: Message,
    ) -> Result<(), Error> {
        while let Some(interaction) = msg
            .await_component_interactions(ctx)
            .timeout(Duration::from_secs(60))
            .author_id(author)
            .stream()
            .next()
            .await
        {
            let custom_id = interaction.data.custom_id.as_str();
            if custom_id == "clan_top_left" {
                self.ranks_index -= 1;
            } else if custom_id == "clan_top_right" {
                self.ranks_index += 1;
            }
            if let Ok(embed) = self.build_embed().await {
                let _r = interaction
                    .create_response(
                        ctx,
                        CreateInteractionResponse::UpdateMessage(
                            CreateInteractionResponseMessage::default()
                                .embed(embed)
                                .components(self.build()),
                        ),
                    )
                    .await;
            }
        }
        let _r = msg
            .edit(
                ctx,
                EditMessage::default().components(self.timeout().build()),
            )
            .await;
        Ok(())
    }

    fn league_name(&self) -> &'static str {
        const LEAGUES: &[&str; 5] = &["Hurricane", "Typhoon", "Storm", "Gale", "Squall"];
        let current_league = self.ranks[self.ranks_index][0];
        LEAGUES[current_league]
    }

    fn division_name(&self) -> &'static str {
        const DIV: &[&str; 3] = &["I", "II", "III"];
        let current_div = self.ranks[self.ranks_index][1];
        DIV[current_div - 1]
    }
    /// get the correponding realm name for api
    pub fn realm(&self, region: &Region) -> &'static str {
        match region {
            Region::Asia => "sg",
            Region::Na => "us",
            Region::Eu => "eu",
        }
    }
    async fn build_embed(&self) -> Result<CreateEmbed, IsacError> {
        let mut embed = CreateEmbed::default()
            .title(format!("{} {}", self.league_name(), self.division_name()))
            .description(format!("{} S{}", self.region.upper(), self.season));
        let res_clans = self.req().await?;
        for clan in res_clans {
            let name = format!(
                "[{}]    rating: {}",
                clan.tag.replace('_', r"\_"),
                clan.division_rating
            );
            let timestamp = DateTime::parse_from_str(&clan.last_battle_at, "%Y-%m-%d %H:%M:%S%z")
                .expect("Failed to parse timestamp")
                .with_timezone(&Utc)
                .timestamp();
            let value = format!("battles: {}, LBT: <t:{}:f>", clan.battles_count, timestamp);
            embed = embed.field(name, value, false);
        }
        Ok(embed)
    }

    fn build(&self) -> Vec<CreateActionRow> {
        let btn_left = CreateButton::new("clan_top_left")
            .custom_id("clan_top_left")
            .emoji(ReactionType::Unicode("◀️".to_string()))
            .style(ButtonStyle::Secondary)
            .disabled(if self.ranks_index == 12 || self.timeout {
                true
            } else {
                false
            });
        let btn_right = CreateButton::new("clan_top_right")
            .emoji(ReactionType::Unicode("▶️".to_string()))
            .style(ButtonStyle::Secondary)
            .disabled(if self.ranks_index == 0 || self.timeout {
                true
            } else {
                false
            });
        vec![CreateActionRow::Buttons(vec![btn_left, btn_right])]
    }

    async fn req(&self) -> Result<Vec<ClanTopLadderClan>, reqwest::Error> {
        let url = self.region.clan_url(format!(
            "/api/ladder/structure/?season={}&realm={}&league={}&division={}",
            self.season,
            self.realm(&self.region),
            self.ranks[self.ranks_index][0],
            self.ranks[self.ranks_index][1]
        ));
        self.client
            .get(url)
            .send()
            .await?
            .json::<Vec<ClanTopLadderClan>>()
            .await
    }

    fn timeout(&mut self) -> &Self {
        self.timeout = true;
        self
    }
}

// https://clans.worldofwarships.asia/api/ladder/structure/?season=21&realm=sg
#[derive(Debug, Deserialize)]
struct ClanTopLadderClan {
    last_battle_at: String, //timestamp like "2023-07-13 13:39:41+00:00"
    tag: ClanTag,
    division_rating: u32,
    battles_count: u32,
}
