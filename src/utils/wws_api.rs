use std::{
    fmt::{format, Display},
    mem,
    sync::Arc,
};

use futures::{stream, StreamExt};
use reqwest::{Client, IntoUrl, Response};
use serde::Deserialize;
use serde_json::Value;
use strum::IntoEnumIterator;

use crate::{
    utils::structs::{Clan, Mode, Player, Region},
    Context,
};

use super::{IsacError, IsacInfo};

// todo: 建立這個struct的成本? 每個internal call都建一個有關係嗎? 真的有必要把他單獨出來?
pub struct WowsApi<'a> {
    pub client: &'a Client,
    token: &'a str,
}

// todo: 這個lifetime聲明怎麼對嗎? 怎麼理解 impl<'a> WowsApi<'a> {}
impl<'a> WowsApi<'a> {
    pub fn new(ctx: &'a Context<'_>) -> WowsApi<'a> {
        Self {
            client: &ctx.data().client,
            token: &ctx.data().wg_api_token,
        }
    }

    /// a shortcut for `client.get()`, wrapped reqwest error into [`IsacInfo::APIError`]
    async fn _get(&self, url: impl IntoUrl) -> Result<Response, IsacError> {
        self.client.get(url).send().await.map_err(|err| {
            IsacInfo::APIError {
                msg: err.to_string(),
            }
            .into()
        })
    }
    /// get player's details with region and uid
    pub async fn player_personal_data(
        &self,
        ctx: &Context<'_>,
        region: Region,
        uid: u64,
    ) -> Result<Player, IsacError> {
        let url = region.vortex_url(format!("/api/accounts/{uid}"))?;

        let res = self._get(url).await?.json::<Value>().await.unwrap();

        Player::parse(ctx.data(), region, res)
    }
    /// searching player with ign
    pub async fn players(
        &self,
        region: &Region,
        ign: &str,
        limit: u32,
    ) -> Result<Vec<VortexPlayer>, IsacError> {
        if ign.len() < 3 {
            Err(IsacInfo::TooShortIgn {
                ign: ign.to_string(),
            })?
        }
        let Ok(url) = region.vortex_url(format!("/api/accounts/search/autocomplete/{ign}/?limit={limit}")) else {
            Err(IsacInfo::InvalidIgn { ign: ign.to_string() })?
        };
        let res = self
            ._get(url)
            .await?
            .json::<VortexPlayerJson>()
            .await
            .unwrap();

        let "ok" = res.status.as_str() else {
            Err(IsacInfo::APIError {
                msg: res.error.unwrap_or_default(),
            })?
        };
        Ok(res.data)
    }
    /// searching clan by its name or tag
    ///
    /// detail: true if you need clan's rename history and members
    pub async fn clans(&self, region: &Region, clan_name: &str) -> Result<Clan, IsacError> {
        let Ok(url) = region.clan_url(format!("/api/search/autocomplete/?search={clan_name}&type=clans")) else {
            Err(IsacInfo::InvalidClan { clan: clan_name.to_string() })?
        };
        let res = self
            ._get(url)
            .await?
            .json::<ClanSearchJson>()
            .await
            .unwrap();
        let clan = res
            .search_autocomplete_result
            .and_then(|mut clans| clans.get_mut(0).map(|clan| mem::take(clan)));

        match clan {
            Some(clan) => Ok(clan.into()),
            None => Err(IsacInfo::ClanNotFound {
                clan: clan_name.to_string(),
                region: *region,
            })?,
        }
    }
    /// get a player clan by his uid
    pub async fn player_clan(&self, region: &Region, player_uid: u64) -> Result<Clan, IsacError> {
        let url = region.vortex_url(format!("/api/accounts/{player_uid}/clans/"))?;
        // again, custom parsing? test url: https://vortex.worldofwarships.asia/api/accounts/2025455227/clans/
        let js_value = self._get(url).await?.json::<Value>().await.unwrap();
        Clan::parse(js_value)
    }

    // wows api doesn't support basic_exp yet, so using vortex still
    // todo: make a builder pattern for wows api
    /// player's all ships stats
    // pub async fn statistics_of_player_ships(
    //     &self,
    //     region: Region,
    //     uid: u64,
    //     ship_id: Option<u64>,
    // ) -> Result<(), IsacError> {
    //     let url = region.api_url("/wows/ships/stats/")?;
    //     let mut query = vec![
    //         ("application_id", self.token.to_string()),
    //         ("lang", "en".to_string()),
    //         ("account_id", uid.to_string()),
    //         ("extra", "pvp_div2, pvp_div3, pvp_solo".to_string()),
    //     ];
    //     if let Some(ship_id) = ship_id {
    //         query.push(("ship_id", ship_id.to_string()));
    //     }
    //     let res = self.client.get(url).query(&query).send();
    // }

    pub async fn statistics_of_player_ships(
        &self,
        region: Region,
        uid: u64,
        ship_id: Option<u32>,
    ) -> Result<(), IsacError> {
        if let Some(ship_id) = ship_id {
            todo!()
        } else {
            // todo: 這個語法...? 來源: https://stackoverflow.com/questions/51044467/how-can-i-perform-parallel-asynchronous-http-get-requests-with-reqwest
            let mut responses = stream::iter(Mode::iter())
                .map(|mode| {
                    let client = self.client.clone();
                    let url = region
                        .vortex_url(format!("/api/accounts/{uid}/ships/{}/", mode.api_name()))
                        .unwrap();
                    tokio::spawn(async move { client.get(url).send().await?.json::<Value>().await })
                })
                .buffer_unordered(5);

            // let mut tank = vec![];
            while let Some(response) = responses.next().await {
                // handle response
                match response {
                    Ok(Ok(res)) => {} // TODO
                    Ok(Err(err)) => Err(IsacError::UnknownError(Box::new(err)))?,
                    Err(err) => Err(IsacError::UnknownError(Box::new(err)))?,
                }
            }
            todo!()
        }
    }
    //     &self,
    //     region: Region,
    //     uid: u64,
    //     ship_id: Option<u64>,
    // ) -> Result<(), IsacError> {
    //     let url = region.api_url("/wows/ships/stats/")?;
    //     let mut query = vec![
    //         ("application_id", self.token.to_string()),
    //         ("lang", "en".to_string()),
    //         ("account_id", uid.to_string()),
    //         ("extra", "pvp_div2, pvp_div3, pvp_solo".to_string()),
    //     ];
    //     if let Some(ship_id) = ship_id {
    //         query.push(("ship_id", ship_id.to_string()));
    //     }
    //     let res = self.client.get(url).query(&query).send();
    // }

    pub async fn clan_detail(&self, clan: Clan) {}
}

pub struct PlayerShipBuilder {}

#[derive(Deserialize, Debug)]
struct VortexPlayerJson {
    pub status: String,
    pub error: Option<String>,
    pub data: Vec<VortexPlayer>,
}

/// the player's data in searching result
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

#[derive(Debug, Deserialize)]
struct ClanSearchJson {
    search_autocomplete_result: Option<Vec<ClanSearchJsonClan>>,
}

/// this is just a temp struct wait for converting to [`Clan`]
#[derive(Debug, Deserialize)]
struct ClanSearchJsonClan {
    id: u64,
    tag: String,
    hex_color: String,
    name: String,
}

impl Default for ClanSearchJsonClan {
    fn default() -> Self {
        Self {
            id: Default::default(),
            tag: Default::default(),
            hex_color: Default::default(),
            name: Default::default(),
        }
    }
}

impl From<ClanSearchJsonClan> for Clan {
    fn from(value: ClanSearchJsonClan) -> Self {
        Clan {
            tag: value.tag,
            color: value.hex_color,
            id: value.id,
            name: value.name,
        }
    }
}
