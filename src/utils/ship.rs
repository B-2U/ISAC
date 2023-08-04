use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};

use crate::utils::structs::Mode;

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub enum ShipClass {
    #[serde(rename = "ss")]
    SS,
    #[serde(rename = "dd")]
    DD,
    #[serde(rename = "ca")]
    CA,
    #[serde(rename = "bb")]
    BB,
    #[serde(rename = "cv")]
    CV,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Ship {
    pub ship_id: u32,
    pub tier: u8,
    pub class: ShipClass,
    pub name: String,
    pub short_name: String,
    pub nation: String,
    pub icon: String,
}
impl Ship {
    /// false for those CB or old ships
    ///
    /// e.g. `Langley (< 23.01.2019)`, `[Moskva]`
    pub fn is_available(&self) -> bool {
        !self.name.contains(['[', '('])
    }
}
impl Display for Ship {
    /// ship's short name
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.short_name)
    }
}

/// the struct for laoding ships_para.json
#[derive(Debug, Deserialize, Serialize)]
pub struct ShipsPara(pub HashMap<u32, Ship>);

impl From<ShipsPara> for HashMap<u32, Ship> {
    fn from(value: ShipsPara) -> Self {
        value.0
    }
}

pub struct ShipStatsCollection(pub HashMap<u32, ShipStats>);

// impl ShipStatsCollection {
//     /// parse ship(s) statistics from vortex json
//     pub fn parse_vortex(region: Region, input: Value) -> Result<Self, IsacError> {
//         Self::_parse_vortex(region, input).map_err(|e| match e.downcast::<IsacError>() {
//             Ok(isac) => *isac,
//             Err(err) => IsacError::UnknownError(err),
//         })
//     }
//     fn _parse_vortex(region: Region, input: Value) -> Result<Self, Error> {
//         let first_layer = input.as_object().unwrap();
//         let "ok" = first_layer.get("status").and_then(|f|f.as_str()).unwrap() else {
//             Err(IsacInfo::APIError { msg: first_layer.get("error").and_then(|f| f.as_str()).unwrap().to_string() })?
//         };
//         let sec_layer = first_layer.get("data").unwrap();
//         if sec_layer.get("hidden_profile").is_some() {
//             Err(IsacInfo::PlayerHidden {
//                 ign: sec_layer
//                     .get("name")
//                     .map(|f| f.as_str())
//                     .flatten()
//                     .unwrap_or("Invalid Player")
//                     .to_string(),
//             })?
//         };
//         let third_layer = sec_layer
//             .as_object()
//             .unwrap()
//             .values()
//             .last()
//             .unwrap()
//             .get("statistics")
//             .unwrap()
//             .as_object()
//             .unwrap();
//         let map = HashMap::new();
//         let ships = third_layer.into_iter().map(|(key, value)| {
//             let key = key.parse::<u32>().unwrap();
//             let ship_stats: ShipStats = serde_json::from_value(value).unwrap();

//         })
//     }
// }

#[derive(Debug, Deserialize)]
pub struct ShipStats(pub HashMap<Mode, Option<ShipStatsMode>>);

#[derive(Debug, Deserialize)]
pub struct ShipStatsMode {
    battles_count: u64,
    wins: u64,
    planes_killed: u64,
    damage_dealt: u64,
    original_exp: u64,
    frags: u64,
    shots_by_main: u64,
    hits_by_main: u64,
    scouting_damage: u64,
}
