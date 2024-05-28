use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};

use crate::utils::LoadSaveFromJson;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ExpectedJs {
    pub time: u64,
    #[serde(deserialize_with = "deserialize_data")]
    pub data: HashMap<u64, ShipExpected>,
}

fn deserialize_data<'de, D>(deserializer: D) -> Result<HashMap<u64, ShipExpected>, D::Error>
where
    D: Deserializer<'de>,
{
    let data: HashMap<u64, serde_json::Value> = Deserialize::deserialize(deserializer)?;
    let mut result: HashMap<u64, ShipExpected> = HashMap::new();

    for (key, value) in data {
        match serde_json::from_value(value) {
            Ok(player_data) => result.insert(key, player_data),
            Err(_) => continue,
        };
    }

    Ok(result)
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
pub struct ShipExpected {
    #[serde(rename = "average_damage_dealt")]
    pub dmg: f64,
    #[serde(rename = "average_frags")]
    pub frags: f64,
    #[serde(rename = "win_rate")]
    pub winrate: f64,
}

impl LoadSaveFromJson for ExpectedJs {
    const PATH: &'static str = "./web_src/ship/expected.json";
}

// impl Default for ExpectedJs {
//     fn default() -> Self {
//         Self::new()
//     }
// }
