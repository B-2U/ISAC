use serde::{Deserialize, Serialize};

use super::Render;
use crate::structs::{PartialClan, Player, StatisticValue};

#[derive(Serialize, Deserialize, Debug)]
pub struct OverallCwTemplate {
    pub seasons: Vec<OverallCwTemplateSeason>,
    pub clan: Option<PartialClan>,
    pub user: Player,
}

impl Render for OverallCwTemplate {
    const RENDER_URL: &'static str = "overall_cw";
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OverallCwTemplateSeason {
    pub season_id: u32,
    pub winrate: StatisticValue,
    pub battles: u64,
    pub dmg: StatisticValue,
    pub frags: StatisticValue,
    pub potential: u64,
    pub scout: u64,
}
