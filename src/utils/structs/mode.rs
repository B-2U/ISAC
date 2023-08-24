use serde::{Deserialize, Serialize};

use std::hash::Hash;
use strum::EnumIter;

#[derive(
    Debug,
    poise::ChoiceParameter,
    Clone,
    Copy,
    EnumIter,
    Deserialize,
    Serialize,
    PartialEq,
    Eq,
    Hash,
    Default,
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
    /// just uppercased
    pub fn upper(&self) -> &'static str {
        match self {
            Mode::Pvp => "PVP",
            Mode::Solo => "SOLO",
            Mode::Div2 => "DIV2",
            Mode::Div3 => "DIV3",
            Mode::Rank => "RANK",
        }
    }
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
    pub fn api_name(&self) -> &'static str {
        match self {
            Mode::Pvp => "pvp",
            Mode::Solo => "pvp_solo",
            Mode::Div2 => "pvp_div2",
            Mode::Div3 => "pvp_div3",
            Mode::Rank => "rank_solo",
        }
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
    pub fn render_name(&self) -> &'static str {
        match self {
            Mode::Pvp => "",
            Mode::Solo => "(solo)",
            Mode::Div2 => "(div2)",
            Mode::Div3 => "(div3)",
            Mode::Rank => "(rank)",
        }
    }
}
