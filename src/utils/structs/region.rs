use crate::utils::{IsacError, LoadFromJson};

use poise::serenity_prelude::GuildId;
use reqwest::Url;
use serde::{Deserialize, Serialize};

use std::{collections::HashMap, fmt::Display, mem};

const GUILD_DEFAULT_PATH: &str = "./user_data/guild_default_region.json";

/// wows server
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum Region {
    #[serde(rename = "asia")]
    Asia,
    #[serde(rename = "na")]
    Na,
    #[serde(rename = "eu")]
    Eu,
}
impl Region {
    pub fn upper(&self) -> String {
        match self {
            Region::Asia => "ASIA",
            Region::Na => "NA",
            Region::Eu => "EU",
        }
        .to_string()
    }
    pub fn lower(&self) -> String {
        self.upper().to_lowercase()
    }
}
impl Display for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.upper())
    }
}

impl Default for Region {
    fn default() -> Self {
        Self::Asia
    }
}

impl Region {
    /// try to parse argument into region, None if none of the regions match
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_lowercase().as_str() {
            "asia" | "sea" => Some(Self::Asia),
            "na" => Some(Self::Na),
            "eu" => Some(Self::Eu),
            _ => None,
        }
    }
    /// return the corresponding vortex url
    pub fn vortex_url(&self, sub_url: impl AsRef<str>) -> Result<Url, IsacError> {
        let base = match self {
            Region::Asia => "https://vortex.worldofwarships.asia",
            Region::Na => "https://vortex.worldofwarships.com",
            Region::Eu => "https://vortex.worldofwarships.eu",
        };
        Self::_construct_url(base, sub_url)
    }
    /// official api url
    pub fn api_url(&self, sub_url: impl AsRef<str>) -> Result<Url, IsacError> {
        let base = match self {
            Region::Asia => "https://api.worldofwarships.asia",
            Region::Na => "https://api.worldofwarships.com",
            Region::Eu => "https://api.worldofwarships.eu",
        };
        Self::_construct_url(base, sub_url)
    }

    /// return the corresponding vortex url
    pub fn number_url(&self, sub_url: impl AsRef<str>) -> Result<Url, IsacError> {
        let base = match self {
            Region::Asia => "https://asia.wows-numbers.com",
            Region::Na => "https://na.wows-numbers.com",
            Region::Eu => "https://wows-numbers.com",
        };
        Self::_construct_url(base, sub_url)
    }

    /// clan api url
    pub fn clan_url(&self, sub_url: impl AsRef<str>) -> Result<Url, IsacError> {
        let base = match self {
            Region::Asia => "https://clans.worldofwarships.asia",
            Region::Na => "https://clans.worldofwarships.com",
            Region::Eu => "https://clans.worldofwarships.eu",
        };
        Self::_construct_url(base, sub_url)
    }

    fn _construct_url(base: &str, sub: impl AsRef<str>) -> Result<Url, IsacError> {
        Url::parse(format!("{}{}", base, sub.as_ref()).as_str())
            .map_err(|err| IsacError::UnknownError(Box::new(err)))
    }
    /// get guild default region setting if exist,
    /// otherwirse return [`Region::Asia`]
    pub async fn guild_default(guild_id: Option<GuildId>) -> Self {
        if let Some(guild_id) = guild_id {
            let mut guild_js: HashMap<_, _> = GuildDefaultRegion::load().await.into();
            match guild_js.get_mut(&guild_id) {
                Some(guild_default) => mem::take(guild_default),
                None => Self::Asia,
            }
        } else {
            // in PM, no guild
            Self::Asia
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GuildDefaultRegion(pub HashMap<GuildId, Region>);

impl GuildDefaultRegion {
    /// load guild json from default path
    ///
    /// # Panics
    /// panic if the path doesn't have available json file
    pub async fn load() -> Self {
        Self::load_json(GUILD_DEFAULT_PATH)
            .await
            .expect(format!("can't find guild_default.json in {GUILD_DEFAULT_PATH}").as_str())
    }
}

impl From<GuildDefaultRegion> for HashMap<GuildId, Region> {
    fn from(value: GuildDefaultRegion) -> Self {
        value.0
    }
}
