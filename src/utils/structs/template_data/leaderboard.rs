use serde::Serialize;

use super::Render;
use crate::utils::structs::{Region, Ship, ShipLeaderboardPlayer};

#[derive(Debug, Serialize, Clone)]
pub struct LeaderboardData {
    pub ship: Ship,
    pub region: Region,
    pub players: Vec<ShipLeaderboardPlayer>,
}

impl Render for LeaderboardData {
    const RENDER_URL: &'static str = "leaderboard";
}
