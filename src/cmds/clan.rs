use std::{
    fmt::Write,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use chrono::DateTime;
use futures::StreamExt;
use itertools::Itertools;
use poise::{
    serenity_prelude::{
        ButtonStyle, CreateActionRow, CreateAttachment, CreateButton, CreateEmbed,
        CreateInteractionResponse, CreateInteractionResponseMessage, EditMessage, Message, UserId,
    },
    CreateReply,
};
use tokio::join;

use crate::{
    dc_utils::{
        autocomplete::{self},
        Args, ContextAddon, EasyEmbed,
    },
    structs::{ClanMember, ClanStatsSeason, PartialClan, StatisticValueType},
    template_data::{
        ClanSeasonTemplate, ClanTemplate, ClanTemplateRename, ClanTemplateSeason,
        ClanTemplateStats, ClanTemplateWrDis, Render,
    },
    utils::{cache_methods, parse, wws_api::WowsApi, IsacError, IsacInfo, LoadSaveFromJson},
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
    #[description = "clan's tag or name, default: yourself"]
    #[autocomplete = "autocomplete::clan"]
    clan: Option<String>,
    #[description = "specify season of Clan Battle, -1 for the latest season"] season: Option<i32>,
) -> Result<(), Error> {
    let api = WowsApi::new(&ctx);
    let partial_clan = if let Some(clan_input) = clan {
        let autocomplete_clan = parse::parse_region_clan(&clan_input)?;
        cache_methods::clan(&api, autocomplete_clan).await?
    } else {
        let author = ctx
            .data()
            .link
            .read()
            .await
            .get(&ctx.author().id)
            .ok_or(IsacError::Info(IsacInfo::UserNotLinked { user_name: None }))?;
        author
            .clan(&api)
            .await
            .ok_or(IsacInfo::UserNoClan { user_name: None })?
    };

    if let Some(season) = season {
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
        let season_num = {
            args.check(0)?
                // for accepting input like `S22`, `s21`
                .trim_matches(|c| c == 's' || c == 'S')
                .parse::<i32>()
                // if it is not a valid positive number, using the latest season as default
                .unwrap_or(0)
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

    // update season if needed
    if let Some(m) = clan_members
        .items
        .first()
        .filter(|m| m.season_id > current_season_num)
    {
        let mut lock = ctx.data().constant.write();
        lock.clan_season = m.season_id;
        lock.save_json_sync();
    }

    let clan_rename = if clan_detail.old_tag.is_some() {
        // old_name and rename_at is possible to be null, thx WG
        let rename_at = clan_detail.renamed_at.unwrap_or(1);
        let old_name = clan_detail.old_name.unwrap_or_default();

        let datetime = DateTime::from_timestamp(rename_at as i64, 0).unwrap();
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
        .send(
            CreateReply::default()
                .attachment(CreateAttachment::bytes(img, "image.png".to_string()))
                .components(view.build())
                .reply(true),
        )
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
    season_num: i32,
) -> Result<(), Error> {
    let current_season_num = ctx.data().constant.read().clan_season;
    let season_num = if season_num >= 0 {
        season_num.unsigned_abs()
    } else if let Some(sub_num) = current_season_num.checked_add_signed(season_num + 1) {
        // negative index, -1 will be the last element
        sub_num
    } else {
        current_season_num
    };

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
        .send(
            CreateReply::default()
                .attachment(CreateAttachment::bytes(img, "image.png".to_string()))
                .reply(true),
        )
        .await?;
    Ok(())
}

struct ClanView {
    clan: PartialClan,
    description: String,
    members: Vec<ClanMember>,
    last_season_btn_disabled: bool,
    timeout: bool,
}

impl ClanView {
    fn new(clan: PartialClan, description: String, members: Vec<ClanMember>) -> Self {
        let description = if description.is_empty() {
            "This clan has no description".to_string()
        } else {
            description
        };
        Self {
            clan,
            description,
            members,
            last_season_btn_disabled: false,
            timeout: false,
        }
    }

    async fn interactions(
        &mut self,
        ctx: &Context<'_>,
        author: UserId,
        mut msg: Message,
    ) -> Result<(), Error> {
        let mut interaction_stream = msg
            .await_component_interactions(ctx)
            .timeout(Duration::from_secs(60))
            .stream();
        while let Some(interaction) = interaction_stream.next().await {
            let custom_id = interaction.data.custom_id.as_str();
            if custom_id == "clan_description" {
                let embed = CreateEmbed::default_isac().description(self.description.clone());
                let _r = interaction
                    .create_response(
                        ctx,
                        poise::serenity_prelude::CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .add_embed(embed)
                                .ephemeral(true),
                        ),
                    )
                    .await;
            } else if custom_id == "clan_members" {
                let embed = CreateEmbed::default_isac().description(self.members_table());
                let _r = interaction
                    .create_response(
                        ctx,
                        poise::serenity_prelude::CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .add_embed(embed)
                                .ephemeral(true),
                        ),
                    )
                    .await;
            } else if custom_id == "latest_season" {
                if interaction.user.id != author {
                    continue;
                };
                // acknowledge the interaction
                let _r = interaction
                    .create_response(ctx, CreateInteractionResponse::Acknowledge)
                    .await;
                // disable the button
                let _r = msg
                    .edit(
                        ctx,
                        EditMessage::default().components(self.pressed().build()),
                    )
                    .await;
                func_clan_season(ctx, self.clan.clone(), -1).await?
            }
        }
        // timeout;
        msg.edit(ctx, EditMessage::new().components(self.timeout().build()))
            .await?;
        Ok(())
    }

    fn members_table(&self) -> String {
        format!(
            "```\n{}\n```",
            self.members
                .iter()
                .sorted_by_key(|m| m.ign.to_lowercase())
                .fold(String::new(), |mut buf, m| {
                    let _ = writeln!(buf, "{}", m.ign);
                    buf
                })
        )
    }

    fn build(&self) -> Vec<CreateActionRow> {
        let mut descrip = CreateButton::new("clan_description")
            .label("Description")
            .style(ButtonStyle::Secondary);
        let mut member = CreateButton::new("clan_members")
            .label("Members")
            .style(ButtonStyle::Secondary);
        let latest_season = CreateButton::new("latest_season")
            .label("Latest season")
            .style(poise::serenity_prelude::ButtonStyle::Secondary)
            .disabled(self.last_season_btn_disabled);
        let official_link = CreateButton::new_link(self.clan.official_url()).label("Official");
        let num_link = CreateButton::new_link(self.clan.wows_number_url()).label("Stats & Numbers");

        if self.timeout {
            descrip = descrip.disabled(true);
            member = member.disabled(true);
        }

        vec![CreateActionRow::Buttons(vec![
            descrip,
            member,
            latest_season,
            official_link,
            num_link,
        ])]
    }

    fn timeout(&mut self) -> &Self {
        self.timeout = true;
        self.last_season_btn_disabled = true;
        self
    }

    /// disabled the season button
    fn pressed(&mut self) -> &Self {
        self.last_season_btn_disabled = true;
        self
    }
}
