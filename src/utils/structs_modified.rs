use crate::{utils::LoadFromJson, Context, Data, Error};

use once_cell::sync::Lazy;
use poise::serenity_prelude::{GuildId, UserId};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::{serde_as, DisplayFromStr};
use std::{collections::HashMap, fmt::Display, hash::Hash, mem};
use strum::EnumIter;

use super::{wws_api::WowsApi, IsacError, IsacInfo};

const LINKED_PATH: &'static str = "./user_data/linked.json";
const GUILD_DEFAULT_PATH: &'static str = "./user_data/guild_default_region.json";
const PFP_PATH: &'static str = "./user_data/pfp.json";

#[derive(Debug, Deserialize)]
struct VortexShip3 {
    statistics: HashMap<ShipId, ShipStats>,
}

#[derive(Debug, Deserialize)]
struct VortexShip2 {
    data: HashMap<String, VortexShip3>,
    name: String,
    hidden_profile: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct VortexShip1 {
    status: String,
    error: Option<String>,
    data: HashMap<String, VortexShip2>,
}

#[derive(Debug, Deserialize, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ShipId(pub u64);
