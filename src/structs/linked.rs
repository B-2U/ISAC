use crate::{structs::PartialPlayer, LoadSaveFromJson};

use poise::serenity_prelude::UserId;

use serde::{Deserialize, Serialize};

use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Linked(pub HashMap<UserId, PartialPlayer>);

impl LoadSaveFromJson for Linked {
    const PATH: &'static str = "./user_data/linked.json";
}

impl Linked {
    /// try to get the discord user linked account, None if not linked
    ///
    /// shortcut for self.0.get().copied()
    pub fn get(&self, user_id: &UserId) -> Option<PartialPlayer> {
        self.0.get(user_id).copied()
    }
}

// impl From<Linked> for HashMap<UserId, PartialPlayer> {
//     fn from(value: Linked) -> Self {
//         value.0
//     }
// }
