use std::{collections::HashSet, fmt::Display, ops::Deref};

use super::{ClanDetail, Region, StatisticValueType, api};
use crate::{
    template_data::{ClanTemplateSeason, ClanTemplateSeasonValue},
    utils::{IsacError, IsacInfo, wws_api::WowsApi},
};

use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_repr::Deserialize_repr;
use serde_with::{DefaultOnError, serde_as};

#[derive(Serialize, Deserialize, Debug, Default, Clone, Hash)]
pub struct PartialClan {
    pub tag: ClanTag,  // e.g. PANTS, do not include [ ]
    pub color: String, // hex color string
    pub id: u64,
    pub name: String,
    pub region: Region,
}

impl Display for PartialClan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.tag)
    }
}

impl PartialClan {
    /// https://asia.wows-numbers.com/clan/2000007634,Dont-Cap-Kill-All/
    pub fn wows_number_url(&self) -> Url {
        self.region.number_url(format!("/clan/{},/", self.id,))
    }
    // TODO: make tests for these urls availability (returned status)
    /// https://clans.worldofwarships.asia/clan-profile/2000017937
    pub fn official_url(&self) -> Url {
        self.region.clan_url(format!("/clan-profile/{}/", self.id))
    }

    pub fn decimal_to_hex(input: u32) -> String {
        format!("{:x}", input)
    }
    /// clan details from vortex, has all CB seasons data
    pub async fn get_clan(&self, api: &WowsApi<'_>) -> Result<Clan, IsacError> {
        api.clan_stats(self.region, self.id).await
    }
    /// clan details from official api, include clan rename history
    pub async fn clan_details(&self, api: &WowsApi<'_>) -> Result<ClanDetail, IsacError> {
        api.clan_details(self.region, self.id).await
    }
    /// clan members from vortex, has members stats
    ///
    /// mode: "pvp", "cvc", default = "pvp"
    /// season only work when mode = "cvc"
    pub async fn clan_members(
        &self,
        api: &WowsApi<'_>,
        mode: Option<&str>,
        season: Option<u32>,
    ) -> Result<ClanMemberAPIRes, IsacError> {
        api.clan_members(self.region, self.id, mode, season).await
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Hash, PartialEq, Eq)]
pub struct ClanTag(String);

impl From<&str> for ClanTag {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl Deref for ClanTag {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for ClanTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ClanTag {
    /// Example: [PANTS]
    pub fn with_brackets(&self) -> String {
        format!("[{}]", self.0)
    }
}
// https://vortex.worldofwarships.asia/api/accounts/2025455227/clans/
/// temp struct waiting for converted to PartialClan
#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerClanAPIRes {
    #[serde(flatten)]
    pub status: api::Status,
    pub error: Option<String>,
    data: Option<PlayerClanData>,
}

impl PlayerClanAPIRes {
    pub fn into_partial_clan(self, region: Region) -> Result<Option<PartialClan>, IsacError> {
        if let Some(err) = self.error {
            // API will return "error": "Not Found" if a player haven't ever joined a clan
            if err == "Not Found" {
                return Ok(None);
            } else {
                Err(IsacInfo::APIError { msg: err })?;
            };
        };
        let data = self.data.expect("should not happen");
        if data.clan_id == 0 {
            Ok(None)
        } else {
            Ok(Some(PartialClan {
                tag: data.clan.tag,
                color: PartialClan::decimal_to_hex(data.clan.color),
                id: data.clan_id,
                name: data.clan.name,
                region,
            }))
        }
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
struct PlayerClanData {
    #[serde_as(deserialize_as = "DefaultOnError")]
    clan: PlayerClanDataClan,
    #[serde_as(deserialize_as = "DefaultOnError")]
    joined_at: String, // "2020-10-10T06:43:53.663284"
    #[serde_as(deserialize_as = "DefaultOnError")]
    role: String, // "recruitment_officer"
    #[serde_as(deserialize_as = "DefaultOnError")]
    clan_id: u64,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct PlayerClanDataClan {
    name: String,
    tag: ClanTag,
    color: u32,
    members_count: u32,
}

// https://clans.worldofwarships.asia/api/clanbase/2000007634/claninfo/
#[derive(Serialize, Deserialize, Debug)]
pub struct ClanInfoAPIRes {
    clanview: Clan,
}

impl From<ClanInfoAPIRes> for Clan {
    fn from(value: ClanInfoAPIRes) -> Self {
        value.clanview
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Clan {
    #[serde(rename(serialize = "info", deserialize = "clan"))]
    pub info: ClanInfo,
    #[serde(rename(serialize = "stats", deserialize = "wows_ladder"))]
    pub stats: ClanStats,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClanInfo {
    pub members_count: u32,
    pub max_members_count: u32,
    pub tag: ClanTag, // e.g. PANTS, do not include [ ]
    pub id: u64,
    pub description: String,
    pub color: String, // hex color string
    pub name: String,
    #[serde(default)]
    pub region: Region, // adding it manually in clan_deatail() after deserialized
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClanStats {
    pub ratings: HashSet<ClanStatsSeason>,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Hash)]
pub struct ClanStatsSeason {
    #[serde(flatten)]
    pub now: ClanStatsRating,
    #[serde(rename(serialize = "max", deserialize = "max_position"))]
    pub max: ClanStatsRating,
    pub battles_count: u32,
    pub wins_count: u32,

    pub current_winning_streak: u32,
    pub longest_winning_streak: u32,
    pub season_number: u32,          // currently 22
    pub team_number: u8,             // should be only 1 or 2
    pub is_best_season_rating: bool, // 2 teams, 1 true 1 false
}
impl ClanStatsSeason {
    /// make a default data with given season
    pub fn default_season(season_num: u32) -> Self {
        Self {
            now: Default::default(),
            max: Default::default(),
            battles_count: 0,
            wins_count: 0,
            current_winning_streak: 0,
            longest_winning_streak: 0,
            season_number: season_num,
            team_number: 0,
            is_best_season_rating: false,
        }
    }
}

impl From<ClanStatsSeason> for ClanTemplateSeason {
    fn from(v: ClanStatsSeason) -> Self {
        ClanTemplateSeason {
            season: v.season_number,
            battles: v.battles_count,
            winrate: StatisticValueType::Winrate {
                value: v.wins_count as f64 / v.battles_count as f64 * 100.0,
            }
            .into(),
            win_streak: v.longest_winning_streak,
            now: v.now.into(),
            max: v.max.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Hash, Default)]
pub struct ClanStatsRating {
    pub league: ClanLeague,     // 0, 1, 2, 3, 4
    pub division: ClanDivision, // 1, 2, 3
    pub division_rating: u32,
    pub public_rating: u32, // the biggest number i saw was about 2600
}
impl From<ClanStatsRating> for ClanTemplateSeasonValue {
    fn from(v: ClanStatsRating) -> Self {
        ClanTemplateSeasonValue {
            color: v.league.color(),
            league: v.league,
            division: v.division,
            division_rating: v.division_rating,
        }
    }
}

#[derive(Serialize, Deserialize_repr, Debug, Eq, PartialEq, Hash, Default)]
#[repr(u8)]
pub enum ClanLeague {
    Hurricane = 0,
    Typhoon = 1,
    Storm = 2,
    Gale = 3,
    #[default]
    Squall = 4,
}
impl ClanLeague {
    pub fn color(&self) -> String {
        match self {
            ClanLeague::Hurricane => "#cda4ff",
            ClanLeague::Typhoon => "#bee7bd",
            ClanLeague::Storm => "#e3d6a0",
            ClanLeague::Gale => "#b3b3b3",
            ClanLeague::Squall => "#cc9966",
        }
        .to_string()
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Serialize, Deserialize_repr, Debug, Eq, PartialEq, Hash, Default)]
#[repr(u8)]
pub enum ClanDivision {
    I = 1,
    II = 2,
    #[default]
    III = 3,
}

// https://clans.worldofwarships.asia/api/members/2000007634/?battle_type=pvp
#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct ClanMemberAPIRes {
    #[serde(flatten)]
    pub status: api::Status,
    #[serde(default)]
    pub items: Vec<ClanMember>,
    #[serde_as(deserialize_as = "DefaultOnError")]
    #[serde(default, rename(deserialize = "clan_statistics"))]
    pub avg: ClanMemberAvgStats,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct ClanMember {
    #[serde_as(deserialize_as = "DefaultOnError")]
    pub battles_per_day: f64,
    pub is_hidden_statistics: bool,
    #[serde(rename = "name")]
    pub ign: String,
    #[serde_as(deserialize_as = "DefaultOnError")]
    pub exp_per_battle: f64,
    #[serde_as(deserialize_as = "DefaultOnError")]
    #[serde(rename(deserialize = "battles_count"))]
    pub battles: u64,
    #[serde_as(deserialize_as = "DefaultOnError")]
    #[serde(rename(deserialize = "wins_percentage"))]
    pub winrate: f64,
    #[serde_as(deserialize_as = "DefaultOnError")]
    #[serde(rename(deserialize = "damage_per_battle"))]
    pub dmg: f64,
    #[serde_as(deserialize_as = "DefaultOnError")]
    pub season_id: u32,
    #[serde_as(deserialize_as = "DefaultOnError")]
    pub last_battle_time: u64, // unix timestamp
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ClanMemberAvgStats {
    pub exp_per_battle: f64,
    #[serde(rename(deserialize = "battles_count"))]
    pub battles: f64,
    /// it * 100.0 already
    #[serde(rename(deserialize = "wins_percentage"))]
    pub winrate: f64,
    #[serde(rename(deserialize = "damage_per_battle"))]
    pub dmg: f64,
    // this field available when battle_type = cvc
    pub ratings: Option<HashSet<ClanStatsSeason>>,
}
