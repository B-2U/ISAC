use serde::Deserialize;

use std::hash::Hash;
use strum::EnumIter;

#[derive(
    Debug, poise::ChoiceParameter, Clone, Copy, EnumIter, Deserialize, PartialEq, Eq, Hash,
)]
pub enum Mode {
    #[serde(rename = "pvp")]
    #[name = "pvp"]
    Pvp,
    #[serde(rename = "pvp_solo")]
    #[name = "solo"]
    Solo,
    #[serde(rename = "pvp_div2")]
    #[name = "div2"]
    Div2,
    #[serde(rename = "pvp_div3")]
    #[name = "div3"]
    Div3,
    #[serde(rename = "rank_solo")]
    #[name = "rank"]
    Rank,
}

impl Mode {
    /// return its name in api
    ///
    /// ## Example
    /// [`Mode::Pvp`] -> pvp
    ///
    /// [`Mode::Solo`] -> pvp_solo
    ///
    /// [`Mode::Div2`] -> pvp_div2
    ///
    /// [`Mode::Div3`] -> pvp_div3
    ///
    /// [`Mode::Rank`] -> rank_solo
    ///
    pub fn api_name(&self) -> String {
        match self {
            Mode::Pvp => "pvp",
            Mode::Solo => "pvp_solo",
            Mode::Div2 => "pvp_div2",
            Mode::Div3 => "pvp_div3",
            Mode::Rank => "rank_solo",
        }
        .to_string()
    }
}
