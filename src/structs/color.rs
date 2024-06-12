use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::utils::IsacError;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ColorStats {
    // #A00DC5 is the SuperUnicum on wows number
    #[serde(rename = "#9D42F3", alias = "#A00DC5")]
    SuperUnicum,
    #[serde(rename = "#D042F3")]
    Unicum,
    #[serde(rename = "#02C9B3")]
    Great,
    #[serde(rename = "#318000")]
    VeryGood,
    #[serde(rename = "#44B300")]
    Good,
    #[serde(rename = "#FFC71F")]
    Average,
    #[serde(rename = "#FE7903")]
    BelowAverage,
    #[serde(rename = "#FE0E00")]
    Bad,
    #[serde(rename = "#999999")]
    Grey,
    #[serde(rename = "#FFFFFF")]
    White,
}

impl FromStr for ColorStats {
    type Err = IsacError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim_matches('"') {
            // #A00DC5 is the SuperUnicum on wows number
            "#9D42F3" | "#A00DC5" => Ok(ColorStats::SuperUnicum),
            "#D042F3" => Ok(ColorStats::Unicum),
            "#02C9B3" => Ok(ColorStats::Great),
            "#318000" => Ok(ColorStats::VeryGood),
            "#44B300" => Ok(ColorStats::Good),
            "#FFC71F" => Ok(ColorStats::Average),
            "#FE7903" => Ok(ColorStats::BelowAverage),
            "#FE0E00" => Ok(ColorStats::Bad),
            _ => Err(IsacError::UnknownError("Unknown value".into())),
        }
    }
}
