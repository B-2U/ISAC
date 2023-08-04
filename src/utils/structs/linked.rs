use crate::{
    utils::{structs::PartialPlayer, LoadFromJson},
    Context, Data, Error,
};

use once_cell::sync::Lazy;
use poise::serenity_prelude::{GuildId, UserId};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::{serde_as, DisplayFromStr};
use std::{collections::HashMap, fmt::Display, hash::Hash, mem};
use strum::EnumIter;

const LINKED_PATH: &'static str = "./user_data/linked.json";
const GUILD_DEFAULT_PATH: &'static str = "./user_data/guild_default_region.json";
const PFP_PATH: &'static str = "./user_data/pfp.json";

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
            .expect(format!("can't find linked.json in {LINKED_PATH}").as_str())
    }
}

impl From<Linked> for HashMap<UserId, PartialPlayer> {
    fn from(value: Linked) -> Self {
        value.0
    }
}
