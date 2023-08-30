use crate::{
    utils::{IsacError, LoadSaveFromJson},
    Context,
};

use poise::serenity_prelude::GuildId;
use reqwest::Url;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

/// wows server
#[derive(
    Default, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, poise::ChoiceParameter, Hash, Eq,
)]
pub enum Region {
    #[default]
    #[serde(rename(serialize = "ASIA", deserialize = "asia"), alias = "ASIA")]
    #[name = "ASIA"]
    Asia,
    #[serde(rename(serialize = "NA", deserialize = "na"), alias = "NA")]
    #[name = "NA"]
    Na,
    #[serde(rename(serialize = "EU", deserialize = "eu"), alias = "EU")]
    #[name = "EU"]
    Eu,
}
impl Region {
    pub fn upper(&self) -> &'static str {
        match self {
            Region::Asia => "ASIA",
            Region::Na => "NA",
            Region::Eu => "EU",
        }
    }
    pub fn lower(&self) -> &'static str {
        match self {
            Region::Asia => "asia",
            Region::Na => "na",
            Region::Eu => "eu",
        }
    }
}
// impl Display for Region {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}", self.upper())
//     }
// }
impl Region {
    /// try to parse argument into region, None if none of the regions match
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_lowercase().as_str() {
            "asia" | "sea" | "aisa" => Some(Self::Asia),
            "na" => Some(Self::Na),
            "eu" => Some(Self::Eu),
            _ => None,
        }
    }
    /// vortex url ( https://vortex.worldofwarships.asia )
    pub fn vortex_url(&self, sub_url: impl AsRef<str>) -> Result<Url, IsacError> {
        let base = match self {
            Region::Asia => "https://vortex.worldofwarships.asia",
            Region::Na => "https://vortex.worldofwarships.com",
            Region::Eu => "https://vortex.worldofwarships.eu",
        };
        Self::_construct_url(base, sub_url)
    }
    /// official api url ( https://api.worldofwarships.asia )
    pub fn api_url(&self, sub_url: impl AsRef<str>) -> Result<Url, IsacError> {
        let base = match self {
            Region::Asia => "https://api.worldofwarships.asia",
            Region::Na => "https://api.worldofwarships.com",
            Region::Eu => "https://api.worldofwarships.eu",
        };
        Self::_construct_url(base, sub_url)
    }

    /// player profile url ( https://profile.worldofwarships.asia )
    pub fn profile_url(&self, sub_url: impl AsRef<str>) -> Result<Url, IsacError> {
        let base = match self {
            Region::Asia => "https://profile.worldofwarships.asia",
            Region::Na => "https://profile.worldofwarships.com",
            Region::Eu => "https://profile.worldofwarships.eu",
        };
        Self::_construct_url(base, sub_url)
    }

    /// number url ( https://asia.wows-numbers.com )
    pub fn number_url(&self, sub_url: impl AsRef<str>) -> Result<Url, IsacError> {
        let base = match self {
            Region::Asia => "https://asia.wows-numbers.com",
            Region::Na => "https://na.wows-numbers.com",
            Region::Eu => "https://wows-numbers.com",
        };
        Self::_construct_url(base, sub_url)
    }

    /// clan api url ( https://clans.worldofwarships.asia )
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

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct GuildDefaultRegion(pub HashMap<GuildId, Region>);

impl GuildDefaultRegion {
    /// get the server default region if exist, return [`Region::Asia`] otherwise
    pub fn get_default(&self, guild_id: Option<GuildId>) -> Region {
        if let Some(guild_id) = guild_id {
            self.0.get(&guild_id).copied().unwrap_or_default()
        } else {
            Region::Asia
        }
    }
}

impl LoadSaveFromJson for GuildDefaultRegion {
    const PATH: &'static str = "./user_data/guild_default_region.json";
}

impl From<GuildDefaultRegion> for HashMap<GuildId, Region> {
    fn from(value: GuildDefaultRegion) -> Self {
        value.0
    }
}
