use crate::{
    utils::{
        structs::{Dogtag, Region},
        wws_api::WowsApi,
        IsacError, IsacInfo, LoadFromJson,
    },
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

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
pub struct PartialPlayer {
    pub region: Region,
    pub uid: u64,
}
impl PartialPlayer {
    pub async fn get_player(&self, ctx: &Context<'_>) -> Result<Player, IsacError> {
        let api = WowsApi::new(&ctx);
        api.player_personal_data(ctx, self.region, self.uid).await
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Player {
    pub uid: u64,
    pub ign: String,
    pub region: Region,
    pub karma: u64,
    pub dog_tag: String,
    pub patch: String,
    pub premium: bool,
    pub pfp: String,
}
// todo: fix this shit code, use serde_with?
impl Player {
    /// parsing player from returned json
    pub fn parse(data: &Data, region: Region, input: Value) -> Result<Player, IsacError> {
        Self::_parse(data, region, input).map_err(|e| match e.downcast::<IsacError>() {
            Ok(isac) => *isac,
            Err(err) => IsacError::UnknownError(err),
        })
    }
    fn _parse(data: &Data, region: Region, input: Value) -> Result<Player, Error> {
        let first_layer = input.as_object().unwrap();
        let "ok" = first_layer.get("status").and_then(|f|f.as_str()).unwrap() else {
            Err(IsacInfo::APIError { msg: first_layer.get("error").and_then(|f| f.as_str()).unwrap().to_string() })?
        };
        let (uid, sec_layer) = first_layer
            .get("data")
            .and_then(|f| f.as_object())
            .unwrap()
            .iter()
            .last()
            .unwrap();
        let uid = uid.parse::<u64>()?;

        let ign = sec_layer
            .get("name")
            .unwrap()
            .as_str()
            .unwrap_or("Invalid Player")
            .to_string();
        if sec_layer.get("hidden_profile").is_some() {
            Err(IsacInfo::PlayerHidden { ign: ign.clone() })?
        }
        let statistics = sec_layer
            .get("statistics")
            .and_then(|f| f.as_object())
            .unwrap();

        let karma = if statistics.len() == 0 {
            Err(IsacInfo::PlayerNoBattle { ign: ign.clone() })?
        } else {
            statistics
                .get("basic")
                .unwrap()
                .get("karma")
                .unwrap()
                .as_u64()
                .unwrap_or_default()
        };
        let dog_tag = sec_layer
            .get("dog_tag")
            .unwrap()
            .get("doll_id")
            .unwrap()
            .as_u64();
        let patch = sec_layer
            .get("dog_tag")
            .unwrap()
            .get("slots")
            .unwrap()
            .get("1")
            .map(|v| v.as_u64())
            .flatten();

        let dog_tag = Dogtag::get(dog_tag).unwrap_or_default();
        let patch = Dogtag::get(patch).unwrap_or_default();
        let premium = data.patron.read().iter().any(|p| p.uid == uid);
        let pfp = if premium {
            let mut pfp_js: HashMap<_, _> = Pfp::load_json_sync(PFP_PATH)?.into();
            pfp_js.remove(&uid).unwrap_or_default().url
        } else {
            "".to_string()
        };

        Ok(Player {
            uid,
            ign,
            region,
            karma,
            dog_tag,
            patch,
            premium,
            pfp,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Pfp(pub HashMap<u64, PfpData>);

impl From<Pfp> for HashMap<u64, PfpData> {
    fn from(value: Pfp) -> Self {
        value.0
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PfpData {
    url: String,
}

impl Default for PfpData {
    fn default() -> Self {
        Self { url: "https://cdn.discordapp.com/attachments/483227767685775360/1117119650052972665/image.png".to_string() }
    }
}
