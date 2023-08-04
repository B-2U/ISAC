mod intermediate;

use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::utils::structs::{ship::intermediate::VortexShipResponse, Mode};

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
#[derive(Debug, Deserialize)]
#[serde(from = "VortexShipResponse")]
pub struct ShipStatsCollection(pub HashMap<u32, ShipModeStatsPair>);

impl From<VortexShipResponse> for ShipStatsCollection {
    fn from(value: VortexShipResponse) -> Self {
        todo!()
    }
}

#[derive(Debug, Deserialize, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ShipId(pub u64);

#[derive(Debug, Deserialize)]
#[serde(try_from = "JsonValue")]
pub struct ShipModeStatsPair(pub HashMap<Mode, Option<ShipStats>>);
impl TryFrom<JsonValue> for ShipModeStatsPair {
    type Error = String;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        let map = value
            .as_object()
            .ok_or("Expected an object (map), but found something else.")?;

        let mut pairs = HashMap::new();
        for (key, value) in map.into_iter() {
            let mode =
                serde_json::from_value(key.as_str().into()).map_err(|err| err.to_string())?;

            // if no stats found for ship, value is an empty object
            let maybe_stats = {
                let stats_map = value
                    .as_object()
                    .ok_or("Expected an object (map), but found something else.")?;

                if stats_map.is_empty() {
                    None
                } else {
                    Some(serde_json::from_value(value.clone()).map_err(|err| err.to_string())?)
                }
            };

            pairs.insert(mode, maybe_stats);
        }

        Ok(Self(pairs))
    }
}

#[derive(Debug, Deserialize)]
pub struct ShipStats {
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
