use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use super::{Region, ShipId, StatisticValue};
use crate::utils::LoadSaveFromJson;

#[derive(Serialize, Deserialize, Default)]
pub struct ShipLeaderboard(pub HashMap<Region, HashMap<ShipId, ShipLeaderboardShip>>);

impl ShipLeaderboard {
    /// get the players on the ship's leaderboard
    pub fn get_ship(
        &mut self,
        region: &Region,
        ship_id: &ShipId,
        timeout_check: bool,
    ) -> Option<Vec<ShipLeaderboardPlayer>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.0
            .entry(*region)
            .or_default()
            .get(ship_id)
            // if `timeout_check` is required && `last_updated_at` over 3600 sec, filter it
            .filter(|ship_cache| !timeout_check || now - ship_cache.last_updated_at <= 3600)
            .map(|ship_cache| ship_cache.players.clone())
    }
    pub fn insert(&mut self, region: &Region, ship_id: ShipId, ship: ShipLeaderboardShip) {
        self.0.entry(*region).or_default().insert(ship_id, ship);
    }
}

impl LoadSaveFromJson for ShipLeaderboard {
    const PATH: &'static str = "./web_src/cache/leaderboard.json";
}

#[derive(Serialize, Deserialize, Default)]
pub struct KokomiShipLeaderboard(pub HashMap<Region, HashMap<ShipId, ShipLeaderboardShip>>);

impl KokomiShipLeaderboard {
    /// get the players on the ship's leaderboard
    pub fn get_ship(
        &mut self,
        region: &Region,
        ship_id: &ShipId,
        timeout_check: bool,
    ) -> Option<Vec<ShipLeaderboardPlayer>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.0
            .entry(*region)
            .or_default()
            .get(ship_id)
            // if `timeout_check` is required && `last_updated_at` over 3600 sec, filter it
            .filter(|ship_cache| !timeout_check || now - ship_cache.last_updated_at <= 3600)
            .map(|ship_cache| ship_cache.players.clone())
    }
    pub fn insert(&mut self, region: &Region, ship_id: ShipId, ship: ShipLeaderboardShip) {
        self.0.entry(*region).or_default().insert(ship_id, ship);
    }
}

impl LoadSaveFromJson for KokomiShipLeaderboard {
    const PATH: &'static str = "./web_src/cache/kokomi_leaderboard.json";
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
    #[serde(default)] // this field in for kokomi leaderboard
    pub exp: StatisticValue,
    // pub planes: StatisticValue, planes got removed from wows number
}
