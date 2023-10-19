use std::{
    borrow::Cow,
    fmt::Write,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use chrono::NaiveDateTime;
use futures::StreamExt;
use itertools::Itertools;
use poise::serenity_prelude::{
    AttachmentType, ButtonStyle, CreateActionRow, CreateButton, CreateComponents, CreateEmbed,
    Message, UserId,
};
use tokio::join;

use crate::{
    dc_utils::{
        auto_complete::{self, AutoCompleteClan},
        Args, ContextAddon, CreateReplyAddon, EasyEmbed, InteractionAddon,
    },
    template_data::{
        ClanSeasonTemplate, ClanTemplate, ClanTemplateRename, ClanTemplateSeason,
        ClanTemplateStats, ClanTemplateWrDis, Render,
    },
    utils::{
        structs::{ClanMember, ClanStatsSeason, PartialClan, StatisticValueType},
        wws_api::WowsApi,
        IsacError, IsacInfo, LoadSaveFromJson,
    },
    Context, Data, Error,
};

pub fn clan_hybrid() -> poise::Command<Data, Error> {
    poise::Command {
        prefix_action: clan_prefix().prefix_action,
        slash_action: clan().slash_action,
        ..clan()
    }
}

/// Clan's overall & CB stats
#[poise::command(slash_command)]
pub async fn clan(
    ctx: Context<'_>,
    #[description = "clan's tag or name"]
    #[autocomplete = "auto_complete::clan"]
    clan: String,
    #[description = "specify season of Clan Battle, -1 for the latest season"] season: Option<i64>,
) -> Result<(), Error> {
    let auto_complete_clan: AutoCompleteClan =
        serde_json::from_str(&clan).map_err(|_| IsacError::Info(IsacInfo::AutoCompleteError))?;
    let api = WowsApi::new(&ctx);
    let partial_clan = api
        .clans(&auto_complete_clan.region, &auto_complete_clan.tag)
        .await?
        .swap_remove(0);

    if let Some(season) = season {
        let current_season_num = ctx.data().constant.read().clan_season;
        let season = match season.is_positive() {
            false => current_season_num,
            true => season.unsigned_abs() as u32,
        };
        func_clan_season(&ctx, partial_clan, season).await
    } else {
        func_clan(&ctx, partial_clan).await
    }
}

#[poise::command(prefix_command)]
pub async fn clan_prefix(ctx: Context<'_>, #[rest] mut args: Args) -> Result<(), Error> {
    let partial_clan = args.parse_clan(&ctx).await?;

    // clan overall
    if args.is_empty() {
        func_clan(&ctx, partial_clan).await
    } else {
        // clan season
        let current_season_num = ctx.data().constant.read().clan_season;
        let season_num = {
            let s = args.check(0)?.parse::<i32>().unwrap_or(-1);
            match s.is_positive() {
                false => current_season_num,
                true => s as u32,
            }
        };
        func_clan_season(&ctx, partial_clan, season_num).await
    }
}

async fn func_clan(ctx: &Context<'_>, partial_clan: PartialClan) -> Result<(), Error> {
    let current_season_num = ctx.data().constant.read().clan_season;
    // QA WowsApi too odd...?
    let api = WowsApi::new(ctx);
    let typing = ctx.typing().await;
    let (clan_detail, clan_members, clan) = join!(
        partial_clan.clan_details(&api),
        partial_clan.clan_members(&api, None, None),
        partial_clan.get_clan(&api)
    );
    // QA 能省掉這三行unwrap嗎?
    let clan_detail = clan_detail?;
    let clan_members = clan_members?;
    let mut clan = clan?;

    // update season_number if needed
    if clan_members
        .items
        .get(0)
        .is_some_and(|m| m.season_id == current_season_num + 1)
    {
        let mut lock = ctx.data().constant.write();
        lock.clan_season += 1;
        lock.save_json_sync();
    }

    let clan_rename = if clan_detail.old_tag.is_some() {
        // old_name and rename_at is possible to be null, thx WG
        let rename_at = clan_detail.renamed_at.unwrap_or(1);
        let old_name = clan_detail.old_name.unwrap_or_default();

        let datetime = NaiveDateTime::from_timestamp_opt(rename_at as i64, 0).unwrap();
        Some(ClanTemplateRename {
            tag: clan_detail.old_tag.unwrap(),
            name: old_name,
            time: datetime.format("%B %e, %Y").to_string(),
        })
    } else {
        None
    };
    let clan_stats = ClanTemplateStats {
        members: clan_members.items.len() as u32,
        active_members: clan_members
            .items
            .iter()
            .filter(|m| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    - m.last_battle_time
                    <= 864000
            })
            .count() as u32,
        winrate: StatisticValueType::Winrate {
            value: clan_members.avg.winrate,
        }
        .into(),
        dmg: StatisticValueType::OverallDmg {
            value: clan_members.avg.dmg,
        }
        .into(),
        exp: clan_members.avg.exp_per_battle.round() as u64,
        wr_dis: ClanTemplateWrDis::sort_wr(&clan_members.items),
    };
    let mut clan_seasons: Vec<ClanTemplateSeason> = clan
        .stats
        .ratings
        .drain()
        // filter only the latest 4 seasons' best rating
        .filter(|f| {
            ((current_season_num - 3)..=current_season_num).contains(&f.season_number)
                && f.is_best_season_rating
        })
        .sorted_by_key(|f| -(f.season_number as i32))
        .map(|f| f.into())
        .collect();
    // fill the missing seasons
    let missing_seasons = ((current_season_num - 3)..=current_season_num)
        .filter(|season| !clan_seasons.iter().any(|s| s.season == *season))
        .map(|season| ClanStatsSeason::default_season(season).into())
        .collect::<Vec<_>>();

    if !missing_seasons.is_empty() {
        clan_seasons.extend(missing_seasons);
        clan_seasons.sort_by(|a, b| b.season.cmp(&a.season));
    }

    let data = ClanTemplate {
        info: partial_clan.clone(),
        seasons: clan_seasons,
        rename: clan_rename,
        stats: clan_stats,
    };
    let img = data.render(&ctx.data().client).await?;
    let mut view = ClanView::new(partial_clan, clan_detail.description, clan_members.items);
    let msg = ctx
        .send(|b| {
            b.attachment(AttachmentType::Bytes {
                data: Cow::Borrowed(&img),
                filename: "image.png".to_string(),
            })
            .set_components(view.build())
            .reply(true)
        })
        .await?
        .into_message()
        .await?;
    typing.stop();
    view.interactions(ctx, ctx.author().id, msg).await?;
    Ok(())
}

async fn func_clan_season(
    ctx: &Context<'_>,
    partial_clan: PartialClan,
    season_num: u32,
) -> Result<(), Error> {
    let api = WowsApi::new(ctx);
    let _typing = ctx.typing().await;
    // QA 下面ratings跟filtered_members魔改clan_members感覺不太好?
    let mut clan_members = partial_clan
        .clan_members(&api, Some("cvc"), Some(season_num))
        .await?;
    // filter out the latest season ratings
    let mut ratings = clan_members
        .avg
        .ratings
        .take()
        .ok_or(IsacError::Info(IsacInfo::ClanNoBattle {
            clan: partial_clan.clone(),
            season: season_num,
        }))?
        .into_iter()
        .filter(|m| m.season_number == season_num)
        .collect::<Vec<_>>();
    // early return if there's no rating
    match ratings.len() {
        0 => Err(IsacError::Info(IsacInfo::ClanNoBattle {
            clan: partial_clan.clone(),
            season: season_num,
        }))?,
        1 => ratings.push(ClanStatsSeason::default_season(season_num)),
        _ => (),
    }
    let filtered_members = clan_members
        .items
        .into_iter()
        .filter(|m| m.battles != 0)
        .collect();

    let data = ClanSeasonTemplate::new(partial_clan, ratings, filtered_members);

    let img = data.render(&ctx.data().client).await?;
    let _msg = ctx
        .send(|b| {
            b.attachment(AttachmentType::Bytes {
                data: Cow::Borrowed(&img),
                filename: "image.png".to_string(),
            })
            .reply(true)
        })
        .await?;
    Ok(())
}

struct ClanView {
    clan: PartialClan,
    description: String,
    members: Vec<ClanMember>,
    last_season_btn: CreateButton,
    timeout: bool,
}

impl ClanView {
    fn new(clan: PartialClan, description: String, members: Vec<ClanMember>) -> Self {
        let btn = CreateButton::default()
            .custom_id("last_season")
            .style(poise::serenity_prelude::ButtonStyle::Secondary)
            .label("Latest season")
            .to_owned();
        let description = if description.is_empty() {
            "❌ This clan has no description".to_string()
        } else {
            description
        };
        Self {
            clan,
            description,
            members,
            last_season_btn: btn,
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
            .build()
            .next()
            .await
        {
            let custom_id = interaction.data.custom_id.as_str();
            if custom_id == "clan_description" {
                let embed = CreateEmbed::isac()
                    .description(self.description.clone())
                    .to_owned();
                let _r = interaction
                    .create_interaction_response(ctx, |i| {
                        i.interaction_response_data(|b| b.add_embed(embed).ephemeral(true))
                    })
                    .await;
            } else if custom_id == "clan_members" {
                let embed = CreateEmbed::isac()
                    .description(self.members_table())
                    .to_owned();
                let _r = interaction
                    .create_interaction_response(ctx, |i| {
                        i.interaction_response_data(|b| b.add_embed(embed).ephemeral(true))
                    })
                    .await;
            } else if custom_id == "last_season" {
                if interaction.user.id != author {
                    continue;
                };
                let _r = interaction
                    .edit_original_message(ctx, |m| m.set_components(self.pressed().build()))
                    .await;
                let current_season_num = ctx.data().constant.read().clan_season;
                func_clan_season(ctx, self.clan.clone(), current_season_num).await?
            }
        }
        // timeout;
        msg.edit(ctx, |m| m.set_components(self.timeout().build()))
            .await?;
        Ok(())
    }

    fn members_table(&self) -> String {
        let members = self
            .members
            .iter()
            .sorted_by_key(|m| m.ign.to_lowercase())
            .fold(String::new(), |mut buf, m| {
                let (winrate, dmg, battles) = if m.battles == 0 {
                    ("-".to_string(), "-".to_string(), "-".to_string())
                } else {
                    (
                        format!("{:.2}%", m.winrate),
                        format!("{:.0}", m.dmg),
                        format!("{}", m.battles),
                    )
                };

                let _ = writeln!(buf, "{:24} {:>6} {:>6} {:>6}", m.ign, winrate, dmg, battles);
                buf
            });
        format!(
            "```{:24} {:>6} {:>6} {:>6}\n{:-<24} {:->6} {:->6} {:->6}\n{members}```",
            "ign", "WR", "DMG", "BTL", "", "", "", ""
        )
    }

    fn build(&self) -> CreateComponents {
        let (des, member, link) = {
            let mut des = CreateButton::default();
            des.label("Description")
                .custom_id("clan_description")
                .style(ButtonStyle::Secondary);
            let mut member = CreateButton::default();
            member
                .label("Members")
                .custom_id("clan_members")
                .style(ButtonStyle::Secondary);
            let mut link = CreateButton::default();
            link.label("Stats & Numbers")
                .url(self.clan.wows_number_url().unwrap())
                .style(ButtonStyle::Link);
            if self.timeout {
                des.disabled(true);
                member.disabled(true);
                link.disabled(true);
            }
            (des, member, link)
        };
        let mut view = CreateComponents::default();
        let mut row = CreateActionRow::default();
        row.add_button(des)
            .add_button(member)
            .add_button(self.last_season_btn.clone())
            .add_button(link);
        view.set_action_row(row);
        view
    }

    fn timeout(&mut self) -> &Self {
        self.timeout = true;
        self.last_season_btn.disabled(true);
        self
    }

    /// disabled the season button
    fn pressed(&mut self) -> &Self {
        self.last_season_btn.disabled(true);
        self
    }
}
