use crate::{utils::LoadFromJson, Context, Data, Error};

use once_cell::sync::Lazy;
use poise::serenity_prelude::{GuildId, UserId};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::{serde_as, DisplayFromStr};
use std::{collections::HashMap, fmt::Display, mem};

use super::{wws_api::WowsApi, IsacError, IsacInfo};

const LINKED_PATH: &'static str = "./user_data/linked.json";
const GUILD_DEFAULT_PATH: &'static str = "./user_data/guild_default_region.json";
const PFP_PATH: &'static str = "./user_data/pfp.json";

#[derive(Deserialize, Debug)]
pub struct Dogtag(pub HashMap<u64, DogtagData>);

impl Dogtag {
    const DOGTAG: Lazy<HashMap<u64, DogtagData>> = Lazy::new(|| {
        Dogtag::load_json_sync("./web_src/dogtag.json")
            .unwrap()
            .into()
    });
    fn get(input: Option<u64>) -> Option<String> {
        let Some(input) = input else {
            return None;
        };
        Self::DOGTAG.get(&input).map(|f| f.icons.small.to_string())
    }
}

impl From<Dogtag> for HashMap<u64, DogtagData> {
    fn from(value: Dogtag) -> Self {
        value.0
    }
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct DogtagData {
    #[serde_as(as = "DisplayFromStr")]
    id: u64,
    title: String,
    icons: DogtagIcon,
}

#[derive(Deserialize, Serialize, Debug)]
struct DogtagIcon {
    small: Url,
    large: Url,
}

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

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
pub struct PartialPlayer {
    pub region: Region,
    pub uid: u64,
}
impl PartialPlayer {
    pub async fn get_player(&self, ctx: &Context<'_>) -> Result<Player, IsacError> {
        let api = WowsApi(&ctx.data().client);
        api.player_personal_data(ctx, &self).await
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Player {
    uid: u64,
    ign: String,
    region: Region,
    karma: u64,
    dog_tag: String,
    patch: String,
    premium: bool,
    pfp: String,
}
// todo: fix this shit code, use serde_with?
impl Player {
    pub fn from(data: &Data, region: Region, input: Value) -> Result<Player, IsacError> {
        Self::_from(data, region, input).map_err(|e| match e.downcast::<IsacError>() {
            Ok(isac) => *isac,
            Err(err) => IsacError::UnkownError(err),
        })
    }
    fn _from(data: &Data, region: Region, input: Value) -> Result<Player, Error> {
        let first_layer = input.as_object().ok_or("parse Player")?;
        let "ok" = first_layer.get("status").and_then(|f|f.as_str()).ok_or("parse Player")? else {
            Err(IsacInfo::APIError { msg: first_layer.get("error").and_then(|f| f.as_str()).ok_or("parse Player")?.to_string() })?
        };
        let (uid, sec_layer) = first_layer
            .get("data")
            .and_then(|f| f.as_object())
            .ok_or("parse Player")?
            .iter()
            .last()
            .ok_or("parse Player")?;
        let uid = uid.parse::<u64>()?;

        let ign = sec_layer
            .get("name")
            .ok_or("parse Player")?
            .as_str()
            .unwrap_or("Invalid Player")
            .to_string();
        if sec_layer.get("hidden_profile").is_some() {
            Err(IsacInfo::PlayerHidden { ign: ign.clone() })?
        }
        let statistics = sec_layer
            .get("statistics")
            .and_then(|f| f.as_object())
            .ok_or("parse Player")?;

        let karma = if statistics.len() == 0 {
            Err(IsacInfo::PlayerNoBattle { ign: ign.clone() })?
        } else {
            statistics
                .get("basic")
                .ok_or("parse Player")?
                .get("karma")
                .ok_or("parse Player")?
                .as_u64()
                .unwrap_or_default()
        };
        let dog_tag = sec_layer
            .get("dog_tag")
            .ok_or("parse Player")?
            .get("doll_id")
            .ok_or("parse Player")?
            .as_u64();
        let patch = sec_layer
            .get("dog_tag")
            .ok_or("parse Player")?
            .get("slots")
            .ok_or("parse Player")?
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

/// wows server
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum Region {
    #[serde(rename = "asia")]
    Asia,
    #[serde(rename = "na")]
    Na,
    #[serde(rename = "eu")]
    Eu,
    #[serde(rename = "ru")]
    Ru,
}
impl Region {
    pub fn upper(&self) -> String {
        match self {
            Region::Asia => "ASIA",
            Region::Na => "NA",
            Region::Eu => "EU",
            Region::Ru => "RU",
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
            "ru" | "cis" => Some(Self::Ru),
            _ => None,
        }
    }
    // todo: remove clone() ?
    /// return the corresponding vortex url
    pub fn vortex(&self) -> Url {
        match self {
            Region::Asia => {
                Lazy::new(|| Url::parse("https://vortex.worldofwarships.asia").unwrap()).clone()
            }
            Region::Na => {
                Lazy::new(|| Url::parse("https://vortex.worldofwarships.com").unwrap()).clone()
            }
            Region::Eu => {
                Lazy::new(|| Url::parse("https://vortex.worldofwarships.eu").unwrap()).clone()
            }
            Region::Ru => Lazy::new(|| Url::parse("https://vortex.korabli.su").unwrap()).clone(),
        }
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
