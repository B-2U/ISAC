use serde::{Deserialize, Serialize};

use crate::utils::structs::{template_data::Render, Mode, PartialClan, Player, Ship, Statistic};

#[derive(Serialize, Deserialize, Debug)]
pub struct RecentTemplate {
    pub clan: PartialClan,
    pub user: Player,
    pub ships: Vec<RecentTemplateShip>,
    pub day: u64,               // the exact_day
    pub main_mode_name: String, // mode.render_name()
    pub main: Statistic,
    pub div: RecentTemplateDiv,
}

impl Render for RecentTemplate {
    const RENDER_URL: &'static str = "recent";
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RecentTemplateDiv {
    pub pvp: Option<Statistic>,
    pub pvp_solo: Option<Statistic>,
    pub pvp_div2: Option<Statistic>,
    pub pvp_div3: Option<Statistic>,
    pub rank_solo: Option<Statistic>,
}

impl RecentTemplateDiv {
    pub fn get_mode(&self, mode: &Mode) -> Option<&Statistic> {
        match mode {
            Mode::Pvp => self.pvp.as_ref(),
            Mode::Solo => self.pvp_solo.as_ref(),
            Mode::Div2 => self.pvp_div2.as_ref(),
            Mode::Div3 => self.pvp_div3.as_ref(),
            Mode::Rank => self.rank_solo.as_ref(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RecentTemplateShip {
    pub info: Ship,
    pub stats: Statistic,
}
