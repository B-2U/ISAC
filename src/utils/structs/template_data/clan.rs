use serde::Serialize;

use super::Render;
use crate::utils::structs::{ClanDivision, ClanLeague, ClanMember, PartialClan, StatisticValue};

#[derive(Debug, Serialize)]
pub struct ClanTemplate {
    pub info: PartialClan,
    pub seasons: Vec<ClanTemplateSeason>,
    pub rename: Option<ClanTemplateRename>,
    pub stats: ClanTemplateStats,
}

impl Render for ClanTemplate {
    const RENDER_URL: &'static str = "clan";
}

#[derive(Debug, Serialize)]
pub struct ClanTemplateStats {
    pub members: u32,
    pub active_members: u32,
    pub winrate: StatisticValue,
    pub dmg: StatisticValue,
    pub exp: u64,
    pub wr_dis: ClanTemplateWrDis,
}

#[derive(Debug, Serialize)]
pub struct ClanTemplateSeason {
    pub season: u32,
    pub battles: u32,
    pub winrate: StatisticValue,
    pub win_streak: u32,
    pub now: ClanTemplateSeasonValue,
    pub max: ClanTemplateSeasonValue,
}

impl ClanTemplateSeason {
    /// make a default data with given season
    pub fn default_season(season_num: u32) -> Self {
        Self {
            season: season_num,
            battles: Default::default(),
            winrate: Default::default(),
            win_streak: Default::default(),
            now: Default::default(),
            max: Default::default(),
        }
    }
}

#[derive(Debug, Serialize, Default)]
pub struct ClanTemplateSeasonValue {
    pub color: String, // hex color, from the league
    pub league: ClanLeague,
    pub division: ClanDivision,
    pub division_rating: u32,
}

#[derive(Debug, Serialize)]
pub struct ClanTemplateRename {
    pub tag: String,
    pub name: String,
    pub time: String,
}

#[derive(Debug, Serialize)]
pub struct ClanTemplateWrDis {
    pub w0: u32,
    pub w45: u32,
    pub w50: u32,
    pub w55: u32,
    pub w60: u32,
    pub w65: u32,
    pub w70: u32,
}

impl ClanTemplateWrDis {
    /// sort players by their winrate
    pub fn sort_wr(members: &[ClanMember]) -> Self {
        let dis = members.iter().fold((0, 0, 0, 0, 0, 0, 0), |mut acc, m| {
            match m.winrate {
                v if v == 0.0 => (), // hidden stats players
                v if v < 45.0 => acc.0 += 1,
                v if v < 50.0 => acc.1 += 1,
                v if v < 55.0 => acc.2 += 1,
                v if v < 60.0 => acc.3 += 1,
                v if v < 65.0 => acc.4 += 1,
                v if v < 70.0 => acc.5 += 1,
                _ => acc.6 += 1,
            }
            acc
        });
        Self {
            w0: dis.0,
            w45: dis.1,
            w50: dis.2,
            w55: dis.3,
            w60: dis.4,
            w65: dis.5,
            w70: dis.6,
        }
    }
}
