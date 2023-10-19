use std::collections::HashSet;

use crate::{
    template_data::{ClanTemplateSeason, ClanTemplateSeasonValue},
    utils::{
        structs::{ClanDetail, Region, StatisticValueType},
        wws_api::WowsApi,
        IsacError, IsacInfo,
    },
};

use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_repr::Deserialize_repr;
use serde_with::{serde_as, DefaultOnError};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct PartialClan {
    pub tag: String,   // e.g. PANTS, do not include [ ]
    pub color: String, // hex color string
    pub id: u64,
    pub name: String,
    pub region: Region,
}

impl PartialClan {
    pub fn wows_number_url(&self) -> Result<Url, IsacError> {
        self.region.number_url(format!(
            "/clan/{},{}/",
            self.id,
            self.name.replace(" ", "-")
        ))
    }

    pub fn decimal_to_hex(input: u64) -> String {
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
    ) -> Result<ClanMemberRes, IsacError> {
        api.clan_members(self.region, self.id, mode, season).await
    }

    pub fn parse(json: Value, region: Region) -> Result<Option<Self>, IsacError> {
        fn err(s: impl AsRef<str>) -> IsacInfo {
            IsacInfo::APIError {
                msg: s.as_ref().into(),
            }
        }
        let "ok" = json.get("status").and_then(|f| f.as_str()).unwrap() else {
            let err_msg = json.get("error").and_then(|f| f.as_str());
            match err_msg {
                Some(err) => Err(IsacInfo::APIError {
                    msg: err.to_string(),
                })?,
                None => Err(IsacInfo::GeneralError {
                    msg: "parsing player's clan failed".to_string(),
                })?,
            }
        };
        let sec_layer = json.get("data").ok_or(err("no data"))?;

        let clan_id = sec_layer.get("clan_id").ok_or(err("no clan_id"))?;
        // not in a clan
        let clan_id = match clan_id.is_u64() {
            true => clan_id.as_u64().ok_or(err("clan_id convert failed"))?,
            false => return Ok(None),
        };

        let third_layer = sec_layer.get("clan").unwrap();
        let name = third_layer
            .get("name")
            .and_then(|f| f.as_str())
            .ok_or(err("no name"))?;
        let tag = third_layer
            .get("tag")
            .and_then(|f| f.as_str())
            .ok_or(err("no tag"))?;
        let color = third_layer
            .get("color")
            .and_then(|f| f.as_u64())
            .ok_or(err("no color"))?;
        // let members_count = third_layer
        //     .get("members_count")
        //     .and_then(|f| f.as_u64())
        //     .unwrap();

        Ok(Some(PartialClan {
            tag: tag.to_string(),
            color: Self::decimal_to_hex(color),
            id: clan_id,
            name: name.to_string(),
            region,
        }))
    }
}

// https://clans.worldofwarships.asia/api/clanbase/2000007634/claninfo/
#[derive(Serialize, Deserialize, Debug)]
pub struct ClanRes {
    clanview: Clan,
}

impl From<ClanRes> for Clan {
    fn from(value: ClanRes) -> Self {
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
    pub tag: String, // e.g. PANTS, do not include [ ]
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
pub struct ClanMemberRes {
    pub status: String,
    pub error: Option<String>,
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
