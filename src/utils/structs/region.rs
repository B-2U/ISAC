use crate::{
    utils::{IsacError, LoadFromJson},
    Context,
};

use poise::serenity_prelude::GuildId;
use reqwest::Url;
use serde::{Deserialize, Serialize};

use std::{collections::HashMap, fmt::Display};

const GUILD_DEFAULT_PATH: &str = "./user_data/guild_default_region.json";

/// wows server
#[derive(Default, Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum Region {
    #[default]
    #[serde(rename(serialize = "ASIA", deserialize = "asia"))]
    Asia,
    #[serde(rename(serialize = "NA", deserialize = "na"))]
    Na,
    #[serde(rename(serialize = "EU", deserialize = "eu"))]
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

    /// player profile url
    pub fn profile_url(&self, sub_url: impl AsRef<str>) -> Result<Url, IsacError> {
        let base = match self {
            Region::Asia => "https://profile.worldofwarships.asia",
            Region::Na => "https://profile.worldofwarships.com",
            Region::Eu => "https://profile.worldofwarships.eu",
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
    pub async fn guild_default(ctx: &Context<'_>) -> Self {
        if let Some(guild_id) = ctx.guild_id() {
            let guild_default_guard = ctx.data().guild_default.read();
            match guild_default_guard.0.get(&guild_id) {
                Some(guild_default) => *guild_default,
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
    pub fn new() -> Self {
        Self::load_json_sync(GUILD_DEFAULT_PATH).unwrap()
    }
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
