use crate::utils::{structs::PartialPlayer, LoadFromJson};

use poise::serenity_prelude::UserId;

use serde::{Deserialize, Serialize};

use std::collections::HashMap;

const LINKED_PATH: &str = "./user_data/linked.json";

#[derive(Serialize, Deserialize, Debug)]
pub struct Linked(pub HashMap<UserId, PartialPlayer>);

impl Linked {
    /// load link json from default path
    ///
    /// # Panics
    /// panic if the path doesn't have available json file
    pub async fn load() -> Self {
        Self::load_json(LINKED_PATH)
            .await
            .unwrap_or_else(|_| panic!("can't find linked.json in {LINKED_PATH}"))
    }
}

impl From<Linked> for HashMap<UserId, PartialPlayer> {
    fn from(value: Linked) -> Self {
        value.0
    }
}
