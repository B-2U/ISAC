use serde::Serialize;

use super::Render;
use crate::utils::structs::{Clan, Player, Ship, Statistic};

#[derive(Debug, Serialize, Clone)]
pub struct ServerTopTemplate {
    pub ship: Ship,
    pub server: String,
    pub players: Vec<ServerTopPlayer>,
}

impl Render for ServerTopTemplate {
    const RENDER_URL: &'static str = "server_top";
}

#[derive(Debug, Serialize, Clone)]
pub struct ServerTopPlayer {
    pub color: String, // hex
    pub rank: usize,
    pub clan: String, // empty if no clan
    pub player: Player,
    pub stats: Statistic,
}
