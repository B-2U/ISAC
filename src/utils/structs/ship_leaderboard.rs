use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::utils::{
    structs::{Region, ShipId, StatisticValue},
    LoadSaveFromJson,
};

#[derive(Serialize, Deserialize)]
pub struct ShipLeaderboard(pub HashMap<Region, HashMap<ShipId, ShipLeaderboardShip>>);

impl ShipLeaderboard {
    ///
    pub fn get_ship(
        &self,
        region: &Region,
        ship_id: &ShipId,
        timeout_check: bool,
    ) -> Option<Vec<ShipLeaderboardPlayer>> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let ship_cache = self.0[region].get(ship_id);
        if ship_cache.is_none()
            || (timeout_check && (timestamp - ship_cache.unwrap().last_updated_at > 3600))
        {
            None
        } else {
            ship_cache.map(|s| s.players.clone())
        }
    }
    pub fn insert(&mut self, region: &Region, ship_id: ShipId, ship: ShipLeaderboardShip) {
        self.0.entry(*region).or_default().insert(ship_id, ship);
    }
}

impl LoadSaveFromJson for ShipLeaderboard {
    const PATH: &'static str = "./web_src/cache/leaderboard.json";
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ShipLeaderboardShip {
    pub players: Vec<ShipLeaderboardPlayer>,
    pub last_updated_at: u64, // unix timestamp
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShipLeaderboardPlayer {
    #[serde(default)]
    pub color: String,
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
