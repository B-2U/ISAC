use poise::serenity_prelude::UserId;
use serde::{Deserialize, Serialize};

use crate::utils::LoadSaveFromJson;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Patrons(pub Vec<Patron>);

impl LoadSaveFromJson for Patrons {
    const PATH: &'static str = "./web_src/cache/patrons.json";
}

impl Patrons {
    pub fn check_user(&self, discord_id: &UserId) -> bool {
        self.0.iter().any(|p| &p.discord_id == discord_id)
    }

    pub fn check_player(&self, uid: &u64) -> bool {
        self.0.iter().any(|p| &p.uid == uid)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Patron {
    pub discord_id: UserId,
    pub uid: u64,
    /// Discord user name
    pub discord_name: String,
    /// Discord nick in the server, None if not set
    pub discord_nick: Option<String>,
}
