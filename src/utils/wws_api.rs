use futures::future::try_join_all;
use reqwest::{Client, IntoUrl, Response, Url};
use serde::Deserialize;
use serde_json::Value;
use std::fmt::Display;
use strum::IntoEnumIterator;

use crate::{
    utils::structs::{
        Clan, ClanDetail, ClanDetailRes, ClanMemberRes, ClanRes, Mode, PartialClan, Player, Region,
        ShipId, ShipStatsCollection, VortexShipResponse,
    },
    Context, Data,
};

use super::{IsacError, IsacInfo};

pub struct WowsApi<'a> {
    pub client: &'a Client,
    token: &'a str,
    data: &'a Data,
}

impl<'a> WowsApi<'a> {
    pub fn new(ctx: &'a Context<'_>) -> WowsApi<'a> {
        Self {
            client: &ctx.data().client,
            token: &ctx.data().wg_api_token,
            data: &ctx.data(),
        }
    }

    /// a shortcut for `client.get()`, wrapped reqwest error into [`IsacInfo::APIError`]
    async fn _get(&self, url: impl IntoUrl) -> Result<Response, IsacError> {
        self.client.get(url).send().await.map_err(Self::_err_wrap)
    }
    /// easily wrapped [`reqwest::Error`] with [`IsacInfo::APIError`]
    fn _err_wrap(err: reqwest::Error) -> IsacError {
        IsacInfo::APIError {
            msg: err.to_string(),
        }
        .into()
    }

    /// get player's details with region and uid
    pub async fn player_personal_data(
        &self,
        region: Region,
        uid: u64,
    ) -> Result<Player, IsacError> {
        let url = region.vortex_url(format!("/api/accounts/{uid}"))?;

        let res = self._get(url).await?.json::<Value>().await.unwrap();

        Player::parse(self.data, region, res).await
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
            .json::<VortexPlayerSearchRes>()
            .await
            .unwrap();

        res.try_into()
    }
    /// searching clan by its name or tag, It will never be a empty vec
    pub async fn clans(
        &self,
        region: &Region,
        clan_name: &str,
    ) -> Result<Vec<PartialClan>, IsacError> {
        let Ok(url) = region.clan_url(format!("/api/search/autocomplete/?search={clan_name}&type=clans")) else {
            Err(IsacInfo::InvalidClan { clan: clan_name.to_string() })?
        };
        let mut res = self._get(url).await?.json::<ClanSearchRes>().await.unwrap();
        let clans = res.search_autocomplete_result.take().map(|clan| {
            clan.into_iter()
                .map(|c| c.to_partial_clan(*region))
                .collect::<Vec<_>>()
        });

        match clans {
            Some(clans) if !clans.is_empty() => Ok(clans),
            _ => Err(IsacInfo::ClanNotFound {
                clan: clan_name.to_string(),
                region: *region,
            })?,
        }
    }
    /// get a player clan by his uid, will return a default clan if the player is not in any clan
    pub async fn player_clan(
        &self,
        region: &Region,
        player_uid: u64,
    ) -> Result<Option<PartialClan>, IsacError> {
        let url = region.vortex_url(format!("/api/accounts/{player_uid}/clans/"))?;
        // again, custom parsing? test url: https://vortex.worldofwarships.asia/api/accounts/2025455227/clans/
        let js_value = self._get(url).await?.json::<Value>().await.unwrap();
        PartialClan::parse(js_value, *region)
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
    //         ("language", "en".to_string()),
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

        let mut ship_stats_merged = try_join_all(requests)
            .await
            .map_err(Self::_err_wrap)?
            .into_iter()
            .map(ShipStatsCollection::try_from)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .reduce(|base, other| base.merge(other))
            .expect("Received 0 responses unexpectedly");

        ship_stats_merged.clean();
        Ok(ship_stats_merged)
    }
    /// clan details from vortex
    pub async fn clan_stats(&self, region: Region, clan_id: u64) -> Result<Clan, IsacError> {
        let url = region.clan_url(format!("/api/clanbase/{clan_id}/claninfo/"))?;
        let mut clan: Clan = self
            ._get(url)
            .await?
            .json::<ClanRes>()
            .await
            .unwrap()
            .into();
        // insert the region here
        clan.info.region = region;
        Ok(clan)
    }

    /// clan members from vortex
    ///
    /// ## mode: "cvc", "pvp"
    pub async fn clan_members(
        &self,
        region: Region,
        clan_id: u64,
        mode: Option<&str>,
        season: Option<u32>,
    ) -> Result<ClanMemberRes, IsacError> {
        let url = region.clan_url(format!("/api/members/{clan_id}/"))?;
        let mut query = vec![("battle_type", mode.unwrap_or("pvp").to_string())];
        if let Some(season) = season {
            query.push(("season", season.to_string()))
        }
        let clan = self
            .client
            .get(url)
            .query(&query)
            .send()
            .await
            .map_err(Self::_err_wrap)?
            .json::<ClanMemberRes>()
            .await
            .unwrap();
        Ok(clan)
    }

    /// clan details from official api
    pub async fn clan_details(
        &self,
        region: Region,
        clan_id: u64,
    ) -> Result<ClanDetail, IsacError> {
        let url = region.api_url(format!("/wows/clans/info/{clan_id}"))?;
        let query = vec![
            ("application_id", self.token.to_string()),
            ("language", "en".to_string()),
            ("clan_id", clan_id.to_string()),
            // ("extra", "members".to_string()),
        ];
        let clan_res: ClanDetailRes = self
            .client
            .get(url)
            .query(&query)
            .send()
            .await
            .map_err(Self::_err_wrap)?
            .json::<ClanDetailRes>()
            .await
            .unwrap()
            .into();
        clan_res.data()
    }
}

#[derive(Deserialize, Debug)]
struct VortexPlayerSearchRes {
    pub status: String,
    pub error: Option<String>,
    #[serde(default)]
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

// QA better way than impl to a Vec<>?
impl TryFrom<VortexPlayerSearchRes> for Vec<VortexPlayerSearch> {
    type Error = IsacError;

    fn try_from(res: VortexPlayerSearchRes) -> Result<Self, Self::Error> {
        if res.status.as_str() != "ok" {
            Err(IsacInfo::APIError {
                msg: res.error.unwrap_or_default(),
            })?
        } else {
            Ok(res.data)
        }
    }
}

#[derive(Debug, Deserialize)]
struct ClanSearchRes {
    search_autocomplete_result: Option<Vec<ClanSearchResClan>>,
}

/// this is just a temp struct wait for converting to [`PartialClan`]
#[derive(Debug, Deserialize, Default)]
struct ClanSearchResClan {
    id: u64,
    tag: String,
    hex_color: String,
    name: String,
}

impl ClanSearchResClan {
    fn to_partial_clan(self, region: Region) -> PartialClan {
        PartialClan {
            tag: self.tag,
            color: self.hex_color,
            id: self.id,
            name: self.name,
            region,
        }
    }
}
