use crate::{
    Context,
    structs::{
        Dogtag, PartialClan, PlayerClanBattle, Region, Ship, ShipModeStatsPair,
        ShipStatsCollection, api,
    },
    utils::{IsacError, IsacInfo, LoadSaveFromJson, cache_methods, wws_api::WowsApi},
};

use once_cell::sync::Lazy;
use poise::serenity_prelude::UserId;
use regex::Regex;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::{DefaultOnError, serde_as};

use std::{collections::HashMap, ops::Deref, str::FromStr};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
pub struct PartialPlayer {
    pub region: Region,
    pub uid: u64,
}
impl PartialPlayer {
    /// check if the players is premium
    pub async fn is_premium(&self, ctx: &Context<'_>) -> bool {
        ctx.data().banner.read().await.get(&self.uid).is_some()
    }

    /// turn partial player into [`Player`]
    /// # Errors
    /// [`IsacInfo::PlayerHidden`] if profile is hidden  
    /// [`IsacInfo::PlayerNoBattle`] if battles = 0
    pub async fn full_player(&self, api: &WowsApi<'_>) -> Result<Player, IsacError> {
        api.player_personal_data(self.region, self.uid).await
    }
    /// the link of player's wow-number page
    pub fn wows_number_url(&self) -> Url {
        self.region.number_url(format!("/player/{},/", self.uid))
    }
    /// the link of player's official profile
    pub fn profile_url(&self) -> Url {
        self.region.profile_url(format!("/statistics/{}", self.uid))
    }
    /// player's clan data
    ///
    /// # Errors
    /// [`IsacInfo::UserNoClan`]  
    /// [`IsacInfo::APIError`]
    pub async fn clan(&self, api: &WowsApi<'_>) -> Result<PartialClan, IsacError> {
        api.player_clan(&self.region, self.uid)
            .await?
            .ok_or(IsacInfo::UserNoClan.into())
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
    ) -> Result<Option<ShipModeStatsPair>, IsacError> {
        let ship_pair = api
            .statistics_of_player_ships(self.region, self.uid, Some(ship.ship_id))
            .await?
            .get_ship(&ship.ship_id);
        Ok(ship_pair)
    }

    pub async fn clan_battle_season_stats(
        &self,
        api: &WowsApi<'_>,
    ) -> Result<PlayerClanBattle, IsacError> {
        api.clan_battle_season_stats(self.region, self.uid).await
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
    pub banner: String,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            partial_player: Default::default(),
            uid: Default::default(),
            ign: "Unknown_Player".to_string(),
            region: Default::default(),
            karma: Default::default(),
            dogtag: Default::default(),
            dogtag_bg: Default::default(),
            premium: Default::default(),
            banner: Default::default(),
        }
    }
}

// QA 到底怎麼讓 Player 繼承 PartialPlayer 的方法?
impl Deref for Player {
    type Target = PartialPlayer;

    fn deref(&self) -> &Self::Target {
        &self.partial_player
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VortexPlayerAPIRes {
    #[serde(flatten)]
    status: api::Status,
    pub data: HashMap<String, VortexPlayer>, // key is player UID. Don't care.
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
pub struct VortexPlayer {
    pub name: String, // player ign
    #[serde(default)]
    pub hidden_profile: bool, // true if hidden, false when not present

    /*
    If hidden_profile == true, all these rest fields won't present
    */
    #[serde_as(deserialize_as = "DefaultOnError")]
    #[serde(default)]
    pub dog_tag: PlayerDogTag,
    #[serde(default)]
    pub statistics: HashMap<String, serde_json::Value>, // we only use whether it's empty or not to check NoBattle
    #[serde(default)]
    pub created_at: f64, // useless
    #[serde(default)]
    pub activated_at: f64, // useless
    #[serde(default)]
    pub visibility_settings: serde_json::Value, // useless
}

impl TryFrom<VortexPlayerAPIRes> for VortexPlayer {
    type Error = IsacError;

    fn try_from(value: VortexPlayerAPIRes) -> Result<Self, Self::Error> {
        value.status.error_for_status()?;
        Ok(value.data.into_iter().next().unwrap().1)
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
    pub fn get_symbol(&self) -> String {
        Dogtag::get(self.symbol_id).unwrap_or_default()
    }
    /// get the background icon url, return empty string if not found
    pub fn get_background(&self) -> String {
        Dogtag::get(self.background_id).unwrap_or_default()
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Banner(pub HashMap<u64, BannerData>);

impl LoadSaveFromJson for Banner {
    const PATH: &'static str = "./user_data/banner.json";
}

impl From<Banner> for HashMap<u64, BannerData> {
    fn from(value: Banner) -> Self {
        value.0
    }
}

impl Banner {
    /// a shortcut to self.0.get(), and auto clone
    pub fn get(&self, uid: &u64) -> Option<BannerData> {
        self.0.get(uid).cloned()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BannerData {
    pub url: String,
    pub name: String, // user name, its just for checking
    pub discord_id: UserId,
}

impl Default for BannerData {
    fn default() -> Self {
        const DEFAULT_BANNER: &str = "./user_data/banner/patreon_default.png";
        Self {
            url: DEFAULT_BANNER.to_string(),
            name: "".to_string(),
            discord_id: UserId::new(0),
        }
    }
}

// a intermediate struct for receiving formatted String from `autocomplete::player()`
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AutocompletePlayer {
    pub region: Region,
    pub ign: String,
}

impl FromStr for AutocompletePlayer {
    type Err = IsacError;

    /// parsing region and ign from str, for example: `[ASIA] B2U` or `ASIA B2U`
    /// # Error
    /// [`IsacInfo::GeneralError`] if received a malformed input
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\w+)\s+\((\w+)\)").unwrap());
        let (_, [ign, region_str]) = RE
            .captures(input)
            .ok_or(IsacInfo::GeneralError {
                msg: "invalid input!".to_string(),
            })?
            .extract();
        let region = Region::parse(region_str).ok_or(IsacInfo::GeneralError {
            msg: format!("`{region_str}` is not a valid region"),
        })?;
        Ok(Self {
            region,
            ign: ign.to_string(),
        })
    }
}

impl From<AutocompletePlayer> for String {
    fn from(val: AutocompletePlayer) -> Self {
        format!("{}  ({})", val.ign, val.region)
    }
}

impl From<AutocompletePlayer> for Value {
    fn from(val: AutocompletePlayer) -> Self {
        Value::String(val.into())
    }
}

impl AutocompletePlayer {
    /// fetch the PartialPlayer
    pub async fn fetch_partial_player(
        &self,
        api: &WowsApi<'_>,
    ) -> Result<PartialPlayer, IsacError> {
        cache_methods::player(api, &self.region, &self.ign).await
    }
    pub async fn save_user_search_history(&self, ctx: &Context<'_>) {
        cache_methods::save_user_search_history(ctx, self.region, self.ign.clone()).await
    }
}
