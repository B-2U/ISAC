use std::{fmt::Display, mem};

use futures::future::try_join_all;
use reqwest::{Client, IntoUrl, Response, Url};
use serde::Deserialize;
use serde_json::Value;
use strum::IntoEnumIterator;

use crate::{
    utils::structs::{Clan, Mode, Player, Region, ShipId, ShipStatsCollection, VortexShipResponse},
    Context,
};

use super::{IsacError, IsacInfo};

pub struct WowsApi<'a> {
    pub client: &'a Client,
    token: &'a str,
}

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

        Player::parse(ctx.data(), region, res).await
    }
    /// searching player with ign
    pub async fn players(
        &self,
        region: &Region,
        ign: &str,
        limit: u32,
    ) -> Result<Vec<VortexPlayerSearch>, IsacError> {
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
            .json::<VortexPlayerSearchResponse>()
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
            .and_then(|mut clans| clans.get_mut(0).map(mem::take));

        match clan {
            Some(clan) => Ok(clan.into()),
            None => Err(IsacInfo::ClanNotFound {
                clan: clan_name.to_string(),
                region: *region,
            })?,
        }
    }
    /// get a player clan by his uid, will return a default clan if the player is not in any clan
    pub async fn player_clan(&self, region: &Region, player_uid: u64) -> Result<Clan, IsacError> {
        let url = region.vortex_url(format!("/api/accounts/{player_uid}/clans/"))?;
        // again, custom parsing? test url: https://vortex.worldofwarships.asia/api/accounts/2025455227/clans/
        let js_value = self._get(url).await?.json::<Value>().await.unwrap();
        Ok(Clan::try_from(js_value)?) // will return a default clan if the player is not in any clan
    }

    // wows api doesn't support basic_exp yet, so using vortex still
    // TODO: make a builder pattern for wows api
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
    /// if `ship_id` is None, it will return all ships statistics
    pub async fn statistics_of_player_ships(
        &self,
        region: Region,
        uid: u64,
        ship_id: Option<ShipId>,
    ) -> Result<ShipStatsCollection, IsacError> {
        let urls: Vec<Url> = if let Some(ship_id) = ship_id {
            Mode::iter()
                .map(|mode| {
                    region
                        .vortex_url(format!(
                            "/api/accounts/{uid}/ships/{ship_id}/{}/",
                            mode.api_name()
                        ))
                        .unwrap()
                })
                .collect()
        } else {
            Mode::iter()
                .map(|mode| {
                    region
                        .vortex_url(format!("/api/accounts/{uid}/ships/{}/", mode.api_name()))
                        .unwrap()
                })
                .collect()
        };
        let requests: Vec<_> = urls
            .into_iter()
            .map(|url| {
                let client = self.client.clone();
                async move {
                    client
                        .get(url)
                        .send()
                        .await?
                        .json::<VortexShipResponse>()
                        .await
                }
            })
            .collect();

        let ship_stats_merged = try_join_all(requests)
            .await
            .map_err(|err| IsacError::UnknownError(Box::new(err)))?
            .into_iter()
            .map(ShipStatsCollection::try_from)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .reduce(|base, other| base.merge(other))
            .expect("Received 0 responses unexpectedly");

        Ok(ship_stats_merged)
    }
    pub async fn clan_detail(&self, _clan: Clan) {}
}

#[derive(Deserialize, Debug)]
struct VortexPlayerSearchResponse {
    pub status: String,
    pub error: Option<String>,
    pub data: Vec<VortexPlayerSearch>,
}

/// the player's data in searching result
#[derive(Deserialize, Debug)]
pub struct VortexPlayerSearch {
    #[serde(rename = "spa_id")]
    pub uid: u64,
    pub name: String,
    pub hidden: bool,
}
impl Display for VortexPlayerSearch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.name.replace('_', r"\_"))
    }
}

#[derive(Debug, Deserialize)]
struct ClanSearchJson {
    search_autocomplete_result: Option<Vec<ClanSearchJsonClan>>,
}

/// this is just a temp struct wait for converting to [`Clan`]
#[derive(Debug, Deserialize, Default)]
struct ClanSearchJsonClan {
    id: u64,
    tag: String,
    hex_color: String,
    name: String,
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
