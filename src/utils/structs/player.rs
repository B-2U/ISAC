use crate::{
    utils::{
        structs::{Clan, Dogtag, Region, Ship, ShipStatsCollection},
        wws_api::WowsApi,
        IsacError, IsacInfo, LoadFromJson,
    },
    Context, Data,
};

use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::{collections::HashMap, ops::Deref};

const PFP_PATH: &str = "./user_data/pfp.json";

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
pub struct PartialPlayer {
    pub region: Region,
    pub uid: u64,
}
impl PartialPlayer {
    /// turn partial player into [`Player`]
    pub async fn get_player(&self, ctx: &Context<'_>) -> Result<Player, IsacError> {
        let api = WowsApi::new(ctx);
        api.player_personal_data(ctx, self.region, self.uid).await
    }
    /// the link of player's wow-number page
    pub fn wows_number_url(&self) -> Result<Url, IsacError> {
        self.region.number_url(format!("/player/{},/", self.uid))
    }
    /// the link of player's official profile
    pub fn profile_url(&self) -> Result<Url, IsacError> {
        self.region.profile_url(format!("/statistics/{}", self.uid))
    }
    /// player's clan data
    pub async fn clan(&self, ctx: &Context<'_>) -> Result<Clan, IsacError> {
        let api = WowsApi::new(ctx);
        api.player_clan(&self.region, self.uid).await
    }
    /// all ships' statistics
    pub async fn all_ships(&self, ctx: &Context<'_>) -> Result<ShipStatsCollection, IsacError> {
        let api = WowsApi::new(ctx);
        api.statistics_of_player_ships(self.region, self.uid, None)
            .await
    }
    /// specific ship's statistics
    pub async fn single_ship(
        &self,
        ctx: &Context<'_>,
        ship: &Ship,
    ) -> Result<ShipStatsCollection, IsacError> {
        let api = WowsApi::new(ctx);
        api.statistics_of_player_ships(self.region, self.uid, Some(ship.ship_id))
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    #[serde(skip_serializing)]
    partial_player: PartialPlayer,
    pub uid: u64,
    pub ign: String,
    pub region: Region,
    pub karma: u64,
    pub dog_tag: String, // might be emblem or dogtag
    pub patch: String,   // the patch on dotag, should be optional
    pub premium: bool,
    pub pfp: String,
}

impl Deref for Player {
    type Target = PartialPlayer;

    fn deref(&self) -> &Self::Target {
        &self.partial_player
    }
}
// TODO fix this shit code, use serde_with?
impl Player {
    /// parsing player from returned json
    pub fn parse(data: &Data, region: Region, input: Value) -> Result<Player, IsacError> {
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
        let uid = uid.parse::<u64>().unwrap();

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

        let karma = if statistics.is_empty() {
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
            .and_then(|v| v.as_u64());

        let dog_tag = Dogtag::get(dog_tag).unwrap_or_default();
        let patch = Dogtag::get(patch).unwrap_or_default();
        let premium = data.patron.read().iter().any(|p| p.uid == uid);
        let pfp = if premium {
            let mut pfp_js: HashMap<_, _> = Pfp::load_json_sync(PFP_PATH).unwrap().into();
            pfp_js.remove(&uid).unwrap_or_default().url
        } else {
            "".to_string()
        };

        Ok(Player {
            partial_player: PartialPlayer { region, uid },
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