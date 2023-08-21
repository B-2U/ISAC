use serde::Serialize;

use super::Render;
use crate::utils::{
    structs::{
        template_data::ClanTemplateSeason, ClanMember, ClanStatsSeason, PartialClan,
        StatisticValue, StatisticValueType,
    },
    IsacError, IsacInfo,
};

#[derive(Debug, Serialize)]
pub struct ClanSeasonTemplate {
    pub info: PartialClan,
    pub alpha: ClanTemplateSeason,
    pub bravo: ClanTemplateSeason,
    pub members_left: Vec<ClanSeasonTemplateMember>,
    pub members_right: Vec<ClanSeasonTemplateMember>,
}

impl Render for ClanSeasonTemplate {
    const RENDER_URL: &'static str = "clan_season";
}

impl ClanSeasonTemplate {
    /// ratings should contain 2 [`ClanStatsSeason`]
    pub fn new(
        partial_clan: PartialClan,
        mut ratings: Vec<ClanStatsSeason>,
        mut members: Vec<ClanMember>,
        season_num: u32,
    ) -> Result<Self, IsacError> {
        // QA, optimizable here?
        let (Some(sec_season), Some(first_season)) = (ratings.pop(), ratings.pop()) else {
            Err(IsacInfo::ClanNoBattle { clan: partial_clan.clone(), season: season_num})?
        };
        let (alpha, bravo) = match first_season.is_best_season_rating {
            true => (first_season, sec_season),
            false => (sec_season, first_season),
        };
        members.sort_by_key(|m| -(m.battles as i64));
        members.truncate(20);
        let (left, right) = Self::_seperate_members(members);
        Ok(Self {
            info: partial_clan,
            alpha: alpha.into(),
            bravo: bravo.into(),
            // QA 就算impl了From Iterator之類的trait 內部也一樣是map into實現的?
            members_left: left.into_iter().map(|m| m.into()).collect(),
            members_right: right.into_iter().map(|m| m.into()).collect(),
        })
        // let alpha = ratings
    }

    fn _seperate_members(mut members: Vec<ClanMember>) -> (Vec<ClanMember>, Vec<ClanMember>) {
        let len = members.len();
        let half = match len % 2 == 0 {
            true => len / 2,
            false => (len / 2) + 1,
        };
        let sec_half = members.split_off(half);
        (members, sec_half)
    }
}

#[derive(Debug, Serialize)]
pub struct ClanSeasonTemplateMember {
    pub ign: String,
    pub battles: u64,
    pub winrate: StatisticValue,
}

impl From<ClanMember> for ClanSeasonTemplateMember {
    fn from(v: ClanMember) -> Self {
        Self {
            ign: v.ign,
            battles: v.battles,
            winrate: StatisticValueType::Winrate { value: v.winrate }.into(),
        }
    }
}
