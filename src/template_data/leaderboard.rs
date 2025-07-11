use serde::Serialize;

use super::Render;
use crate::structs::{Region, Ship, ShipLeaderboardPlayer};

#[derive(Debug, Serialize, Clone)]
pub struct LeaderboardTemplate {
    pub ship: Ship,
    pub region: Region,
    pub players: Vec<ShipLeaderboardPlayer>,
}

impl Render for LeaderboardTemplate {
    const RENDER_URL: &'static str = "leaderboard";
}

#[derive(Debug, Serialize, Clone)]
pub struct KLeaderboardTemplate(pub LeaderboardTemplate);

impl Render for KLeaderboardTemplate {
    const RENDER_URL: &'static str = "kleaderboard";
}
