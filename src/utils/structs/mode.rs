use serde::Deserialize;

use std::hash::Hash;
use strum::EnumIter;

#[derive(
    Debug, poise::ChoiceParameter, Clone, Copy, EnumIter, Deserialize, PartialEq, Eq, Hash, Default,
)]
pub enum Mode {
    #[serde(rename = "pvp")]
    #[default]
    Pvp,
    #[serde(rename = "pvp_solo")]
    Solo,
    #[serde(rename = "pvp_div2")]
    Div2,
    #[serde(rename = "pvp_div3")]
    Div3,
    #[serde(rename = "rank_solo")]
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

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value.to_lowercase().as_str() {
            "pvp" => Self::Pvp,
            "solo" => Self::Solo,
            "div2" => Self::Div2,
            "div3" => Self::Div3,
            "rank" | "ranked" => Self::Rank,
            _ => None?,
        })
    }
    /// for showing to user in images
    ///
    /// **Note** [`Mode::Pvp`] will return an empty String
    ///
    /// ## Example
    /// [`Mode::Pvp`] ->
    ///
    /// [`Mode::Solo`] -> (solo)
    ///
    /// [`Mode::Div2`] -> (div2)
    ///
    /// [`Mode::Div3`] -> (div3)
    ///
    /// [`Mode::Rank`] -> (rank)
    ///
    pub fn display_name(&self) -> String {
        match self {
            Mode::Pvp => "",
            Mode::Solo => "(solo)",
            Mode::Div2 => "(div2)",
            Mode::Div3 => "(div3)",
            Mode::Rank => "(rank)",
        }
        .to_string()
    }
}
