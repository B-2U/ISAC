use crate::utils::{structs::PartialPlayer, LoadSaveFromJson};

use poise::serenity_prelude::UserId;

use serde::{Deserialize, Serialize};

use std::collections::HashMap;

const LINKED_PATH: &str = "./user_data/linked.json";

#[derive(Serialize, Deserialize, Debug)]
pub struct Linked(pub HashMap<UserId, PartialPlayer>);

impl LoadSaveFromJson for Linked {
    const PATH: &'static str = "./user_data/linked.json";
}

impl From<Linked> for HashMap<UserId, PartialPlayer> {
    fn from(value: Linked) -> Self {
        value.0
    }
}
