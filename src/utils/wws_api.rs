use std::fmt::Display;

use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use crate::Context;

use super::{
    user::{PartialPlayer, Player, Region},
    IsacError, IsacInfo,
};

// todo: is this good code?(wrapped client in new struct),
pub struct WowsApi<'a>(pub &'a Client);

impl WowsApi<'_> {
    pub async fn player_personal_data(
        &self,
        ctx: &Context<'_>,
        partial_player: &PartialPlayer,
    ) -> Result<Player, IsacError> {
        let url = partial_player
            .region
            .vortex()
            .join(format!("api/accounts/{}", partial_player.uid).as_str())
            .unwrap();

        let res = match self.0.get(url).send().await {
            Ok(res) => res,
            Err(err) => Err(IsacError::Info(IsacInfo::APIError {
                msg: err.to_string(),
            }))?,
        }
        .json::<Value>()
        .await
        .unwrap();

        Player::from(ctx.data(), partial_player.region, res)
    }
    pub async fn players(
        &self,
        region: &Region,
        ign: &str,
        limit: u32,
    ) -> Result<Vec<VortexPlayer>, IsacError> {
        if ign.len() < 3 {
            Err(IsacError::Info(IsacInfo::TooShortIgn {
                ign: ign.to_string(),
            }))?
        }
        let Ok(url) = region.vortex().join(format!("api/accounts/search/autocomplete/{ign}/?limit={limit}").as_str()) else {
            Err(IsacError::Info(IsacInfo::InvalidIgn { ign: ign.to_string() }))?
        };
        let res = match self.0.get(url).send().await {
            Ok(res) => res,
            Err(err) => Err(IsacError::Info(IsacInfo::APIError {
                msg: err.to_string(),
            }))?,
        }
        .json::<VortexPlayerJson>()
        .await
        .unwrap();

        let "ok" = res.status.as_str() else {
            Err(IsacError::Info(IsacInfo::APIError {
                msg: res.error.unwrap_or_default(),
            }))?
        };
        Ok(res.data)
    }
}

#[derive(Deserialize, Debug)]
struct VortexPlayerJson {
    pub status: String,
    pub error: Option<String>,
    pub data: Vec<VortexPlayer>,
}
#[derive(Deserialize, Debug)]
pub struct VortexPlayer {
    #[serde(rename = "spa_id")]
    pub uid: u64,
    pub name: String,
    pub hidden: bool,
}
impl Display for VortexPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.name.replace("_", r"\_"))
    }
}
