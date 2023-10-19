use crate::{
    utils::{
        structs::{
            Dogtag, PartialClan, Region, Ship, ShipId, ShipModeStatsPair, ShipStatsCollection,
        },
        wws_api::WowsApi,
        IsacError, IsacInfo, LoadSaveFromJson,
    },
    Data,
};

use poise::serenity_prelude::UserId;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::{collections::HashMap, ops::Deref};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
pub struct PartialPlayer {
    pub region: Region,
    pub uid: u64,
}
impl PartialPlayer {
    /// turn partial player into [`Player`]
    pub async fn get_player(&self, api: &WowsApi<'_>) -> Result<Player, IsacError> {
        api.player_personal_data(self.region, self.uid).await
    }
    /// the link of player's wow-number page
    pub fn wows_number_url(&self) -> Result<Url, IsacError> {
        self.region.number_url(format!("/player/{},/", self.uid))
    }
    /// the link of player's official profile
    pub fn profile_url(&self) -> Result<Url, IsacError> {
        self.region.profile_url(format!("/statistics/{}", self.uid))
    }
    /// player's clan data, only error when request or api issue
    pub async fn clan(&self, api: &WowsApi<'_>) -> Result<Option<PartialClan>, IsacError> {
        api.player_clan(&self.region, self.uid).await
    }
    /// all ships' statistics
    pub async fn all_ships(&self, api: &WowsApi<'_>) -> Result<ShipStatsCollection, IsacError> {
        api.statistics_of_player_ships(self.region, self.uid, None)
            .await
    }
    /// specific ship's statistics
    pub async fn single_ship(
        &self,
        api: &WowsApi<'_>,
        ship: &Ship,
    ) -> Result<Option<(ShipId, ShipModeStatsPair)>, IsacError> {
        let ship_pair = api
            .statistics_of_player_ships(self.region, self.uid, Some(ship.ship_id))
            .await?
            .0
            .remove(&ship.ship_id);
        Ok(ship_pair.map(|p| (ship.ship_id, p)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    #[serde(skip_serializing)]
    pub partial_player: PartialPlayer,
    pub uid: u64,
    pub ign: String,
    pub region: Region,
    pub karma: u64,
    pub dogtag: String,    // might be emblem or dogtag
    pub dogtag_bg: String, // the dotag background, should be optional
    pub premium: bool,
    pub pfp: String,
}

// QA 到底怎麼讓 Player 繼承 PartialPlayer 的方法?
impl Deref for Player {
    type Target = PartialPlayer;

    fn deref(&self) -> &Self::Target {
        &self.partial_player
    }
}
// TODO fix this shit code, use serde_with?
impl Player {
    /// parsing player from returned json
    pub async fn parse(data: &Data, region: Region, input: Value) -> Result<Player, IsacError> {
        let first_layer = input.as_object().unwrap();
        let "ok" = first_layer.get("status").and_then(|f| f.as_str()).unwrap() else {
            Err(IsacInfo::APIError {
                msg: first_layer
                    .get("error")
                    .and_then(|f| f.as_str())
                    .unwrap()
                    .to_string(),
            })?
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

        let player_dogtag: PlayerDogTag =
            serde_json::from_value(sec_layer.get("dog_tag").unwrap().clone()).unwrap_or_default();

        let dogtag = player_dogtag.get_symbol();
        let dogtag_bg = player_dogtag.get_background();
        let premium = data.patron.read().check_player(&uid);
        let pfp = if premium {
            let pfp_js = data.pfp.read();
            pfp_js.get(&uid).unwrap_or_default().url
        } else {
            "".to_string()
        };

        Ok(Player {
            partial_player: PartialPlayer { region, uid },
            uid,
            ign,
            region,
            karma,
            dogtag,
            dogtag_bg,
            premium,
            pfp,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct PlayerDogTag {
    texture_id: u64,
    symbol_id: u64,
    border_color_id: u64,
    background_color_id: u64,
    background_id: u64,
}

impl PlayerDogTag {
    /// get the symbol icon url, return empty string if not found
    fn get_symbol(&self) -> String {
        Dogtag::get(self.symbol_id).unwrap_or_default()
    }
    /// get the background icon url, return empty string if not found
    fn get_background(&self) -> String {
        Dogtag::get(self.background_id).unwrap_or_default()
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Pfp(pub HashMap<u64, PfpData>);

impl LoadSaveFromJson for Pfp {
    const PATH: &'static str = "./user_data/pfp.json";
}

impl From<Pfp> for HashMap<u64, PfpData> {
    fn from(value: Pfp) -> Self {
        value.0
    }
}

impl Pfp {
    /// a shortcut to self.0.get(), and auto clone
    pub fn get(&self, uid: &u64) -> Option<PfpData> {
        self.0.get(uid).cloned()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PfpData {
    pub url: String,
    pub name: String, // user name, its just for checking
    pub discord_id: UserId,
}

impl Default for PfpData {
    fn default() -> Self {
        const DEFAULT_PFBG: &'static str = "https://cdn.discordapp.com/attachments/483227767685775360/1117119650052972665/image.png";
        Self {
            url: DEFAULT_PFBG.to_string(),
            name: "".to_string(),
            discord_id: UserId(0),
        }
    }
}
