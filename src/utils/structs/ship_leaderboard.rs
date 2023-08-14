use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    utils::{
        structs::{Region, ShipId, StatisticValue},
        LoadFromJson,
    },
    Error,
};

const SHIP_LEADERBOARD_PATH: &str = "./web_src/cache/leaderboard";

#[derive(Serialize, Deserialize)]
pub struct ShipLeaderboard(pub HashMap<ShipId, ShipLeaderboardShip>);

impl ShipLeaderboard {
    /// laod the specific region's leaderboard cache
    pub async fn load(region: Region) -> Result<Self, Error> {
        // TODO: fix this format shit when building real database
        Self::load_json(format!("{SHIP_LEADERBOARD_PATH}/{}.json", region.lower())).await
    }
}

#[derive(Serialize, Deserialize)]
pub struct ShipLeaderboardShip {
    pub players: Vec<ShipLeaderboardPlayer>,
    pub last_updated_at: u64, // unix timestamp
}

#[derive(Serialize, Deserialize)]
pub struct ShipLeaderboardPlayer {
    pub rank: u64,
    pub clan: String,
    pub ign: String,
    pub uid: u64,
    pub battles: u64,
    pub pr: StatisticValue,
    pub winrate: StatisticValue,
    pub frags: StatisticValue,
    pub dmg: StatisticValue,
    pub planes: StatisticValue,
}
