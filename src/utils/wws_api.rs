use futures::future::try_join_all;
use reqwest::{Client, IntoUrl, Response, Url};
use serde::Deserialize;
use std::fmt::{Debug, Display};
use strum::IntoEnumIterator;
use ureq::typestate::WithoutBody;

use crate::{
    Context, Data,
    structs::{
        AutocompletePlayer, Clan, ClanDetail, ClanDetailAPIRes, ClanInfoAPIRes, ClanMemberAPIRes,
        ClanTag, Mode, PartialClan, PartialPlayer, Player, PlayerClanAPIRes, PlayerClanBattle,
        PlayerClanBattleAPIRes, Region, ShipId, ShipStatsCollection, ShipsPara, VortexPlayer,
        VortexPlayerAPIRes, VortexShipAPIRes, VortexVehicleAPIRes, api,
    },
};

use super::{IsacError, IsacInfo};

#[derive(Clone, Copy)]
pub struct WowsApi<'a> {
    pub client: &'a Client,
    token: &'a str,
    ctx_data: &'a Data,
}

impl<'a> WowsApi<'a> {
    pub fn new(ctx: &'a Context<'_>) -> WowsApi<'a> {
        Self {
            client: &ctx.data().client,
            token: &ctx.data().wg_api_token,
            ctx_data: ctx.data(),
        }
    }

    /// Helper function to handle errors, wrapped reqwest error into [`IsacInfo::APIError`]
    pub async fn reqwest<F>(
        &self,
        url: impl IntoUrl + Display + Clone,
        builder: F,
    ) -> Result<Response, IsacError>
    where
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        let request_builder = self.client.get(url.clone());
        let request_builder = builder(request_builder);
        request_builder
            .send()
            .await
            .map_err(|err| Self::_err_wrap(&url, err))?
            .error_for_status()
            .map_err(|err| Self::_err_wrap(&url, err))
    }

    /// Helper function to handle errors, wrapped ureq error into [`IsacInfo::APIError`]
    pub async fn ureq<F>(
        &self,
        url: impl IntoUrl + Display + Clone,
        builder: F,
    ) -> Result<ureq::Body, IsacError>
    where
        F: FnOnce(ureq::RequestBuilder<WithoutBody>) -> ureq::RequestBuilder<WithoutBody>,
    {
        let url = url.to_string();
        let request_builder = ureq::get(&url);
        let request_builder = builder(request_builder);
        tokio::task::spawn_blocking(|| request_builder.call())
            .await
            .unwrap()
            .map_err(|err| Self::_err_wrap(&url, err))
            .map(|res| res.into_body())
    }

    /// log error and return [`IsacError`]
    fn _err_wrap(url: &impl Display, err: impl Debug + Into<IsacError>) -> IsacError {
        tracing::warn!("url: {}\n{:#?}", url, err);
        err.into()
    }

    /// get player's details with region and uid
    pub async fn player_personal_data(
        &self,
        region: Region,
        uid: u64,
    ) -> Result<Player, IsacError> {
        let url = region.vortex_url(format!("/api/accounts/{uid}"));

        let res: VortexPlayer = self
            .reqwest(url, |b| b)
            .await?
            .json::<VortexPlayerAPIRes>()
            .await?
            .try_into()?;

        if res.hidden_profile {
            return Err(IsacInfo::PlayerHidden { ign: res.name }.into());
        }
        // leveling_points will increase in any mode in any ship (including test ships)
        // use it to determine PlayerNoBattle or not
        if res
            .statistics
            .get("basic")
            .and_then(|d| d.get("leveling_points").and_then(|lv| lv.as_u64()))
            .unwrap_or_default()
            == 0
        {
            return Err(IsacInfo::PlayerNoBattle { ign: res.name }.into());
        }
        let karma = res
            .statistics
            .get("basic")
            .and_then(|v| v.get("karma").and_then(|v2| v2.as_u64()))
            .unwrap_or_default();
        let banner = self.ctx_data.banner.read().await.get(&uid).map(|r| r.url);
        Ok(Player {
            partial_player: PartialPlayer { region, uid },
            uid,
            ign: res.name,
            region,
            karma,
            dogtag: res.dog_tag.get_symbol(),
            dogtag_bg: res.dog_tag.get_background(),
            premium: banner.is_some(),
            banner: banner.unwrap_or_default(),
        })

        // Player::parse(self.data, region, res).await
    }
    /// searching player with ign, return an empty vec if no matched
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
        if !ign.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            Err(IsacInfo::InvalidIgn {
                ign: ign.to_string(),
            })?
        };
        let url = region.vortex_url(format!(
            "/api/accounts/search/autocomplete/{ign}/?limit={limit}"
        ));
        let res = self
            .reqwest(url, |b| b)
            .await?
            .json::<VortexPlayerSearchAPIRes>()
            .await
            .unwrap();

        res.status.error_for_status()?;
        Ok(res.data)
    }
    /// searching clan by its name or tag, It will never be a empty vec
    pub async fn clans(
        &self,
        region: &Region,
        clan_name: &str,
    ) -> Result<Vec<PartialClan>, IsacError> {
        if clan_name.len() <= 1
        // api now requires at least 2 chars
        {
            Err(IsacInfo::InvalidClan {
                clan: clan_name.to_string(),
            })?
        };
        let url = region.clan_url(format!(
            "/api/search/autocomplete/?search={clan_name}&type=clans"
        ));
        let mut res = self
            .reqwest(url, |b| b)
            .await?
            .json::<ClanSearchRes>()
            .await
            .unwrap();
        let clans = res.search_autocomplete_result.take().map(|clan| {
            clan.into_iter()
                .map(|c| c.into_partial_clan(*region))
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
    /// get a player clan by his uid
    pub async fn player_clan(
        &self,
        region: &Region,
        player_uid: u64,
    ) -> Result<Option<PartialClan>, IsacError> {
        let url = region.vortex_url(format!("/api/accounts/{player_uid}/clans/"));
        let res = self
            .reqwest(url, |b| b)
            .await?
            // return None if clan API is fucked
            .json::<PlayerClanAPIRes>()
            .await
            .unwrap();

        res.into_partial_clan(*region)
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
                    region.vortex_url(format!(
                        "/api/accounts/{uid}/ships/{ship_id}/{}/",
                        mode.api_name()
                    ))
                })
                .collect()
        } else {
            Mode::iter()
                .map(|mode| {
                    region.vortex_url(format!("/api/accounts/{uid}/ships/{}/", mode.api_name()))
                })
                .collect()
        };
        let requests: Vec<_> = urls
            .into_iter()
            .map(|url| async move {
                let res = self.reqwest(url.clone(), |b| b).await?;
                res.json::<VortexShipAPIRes>()
                    .await
                    .map_err(|err| Self::_err_wrap(&url, err))
            })
            .collect();

        let mut ship_stats_merged = try_join_all(requests)
            .await?
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
        let url = region.clan_url(format!("/api/clanbase/{clan_id}/claninfo/"));
        let mut clan: Clan = self
            .reqwest(url, |b| b)
            .await?
            .json::<ClanInfoAPIRes>()
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
    ) -> Result<ClanMemberAPIRes, IsacError> {
        let url = region.clan_url(format!("/api/members/{clan_id}/"));
        let mut query = vec![("battle_type", mode.unwrap_or("pvp").to_string())];
        if let Some(season) = season {
            query.push(("season", season.to_string()))
        }
        let clan = self
            .reqwest(url, |b| b.query(&query))
            .await?
            .json::<ClanMemberAPIRes>()
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
        let url = region.api_url(format!("/wows/clans/info/{clan_id}"));
        let query = vec![
            ("application_id", self.token.to_string()),
            ("language", "en".to_string()),
            ("clan_id", clan_id.to_string()),
            // ("extra", "members".to_string()),
        ];

        // // TEMP code for DEBUG here

        // let res = self
        //     .client
        //     .get(url)
        //     .query(&query)
        //     .send()
        //     .await
        //     .and_then(|res| res.error_for_status())
        //     .map_err(Self::_err_wrap)?;
        // let res_status = res.status();
        // let res_text = res.text().await?;

        // match serde_json::from_str::<ClanDetailAPIRes>(&res_text) {
        //     Ok(clan_res) => clan_res.data(),
        //     Err(err) => {
        //         println!("Err code: {}", res_status);
        //         println!("Response: {:?}", res_text);
        //         panic!("{:?}", err)
        //     }
        // }

        let clan_res: ClanDetailAPIRes = self
            .reqwest(url, |b| b.query(&query))
            .await?
            .json::<ClanDetailAPIRes>()
            .await
            .unwrap();
        clan_res.data()
    }

    /// player's CB seasons stats from official api
    pub async fn clan_battle_season_stats(
        &self,
        region: Region,
        uid: u64,
    ) -> Result<PlayerClanBattle, IsacError> {
        let url = region.api_url("/wows/clans/seasonstats/");
        let query = vec![
            ("application_id", self.token.to_string()),
            ("language", "en".to_string()),
            ("account_id", uid.to_string()),
        ];
        let mut res: PlayerClanBattle = self
            .reqwest(url, |b| b.query(&query))
            .await?
            .json::<PlayerClanBattleAPIRes>()
            .await?
            .try_into()?;
        // filter out some ancient clan battle seasons
        res.seasons.retain(|s| !matches!(s.season_id, 101 | 102));
        Ok(res)
    }

    pub async fn encyclopedia_vehicles(&self) -> Result<ShipsPara, IsacError> {
        self.reqwest(
            "https://vortex.worldofwarships.com/api/encyclopedia/en/vehicles/",
            |b| b,
        )
        .await?
        .json::<VortexVehicleAPIRes>()
        .await
        .unwrap()
        .try_into()
    }
}

#[derive(Deserialize, Debug)]
struct VortexPlayerSearchAPIRes {
    #[serde(flatten)]
    pub status: api::Status,
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

impl VortexPlayerSearch {
    /// for turning it into [`AutocompleteChoice`]
    pub fn into_autocomplete_player(self, region: Region) -> AutocompletePlayer {
        AutocompletePlayer {
            region,
            ign: self.name,
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
    tag: ClanTag,
    hex_color: String,
    name: String,
}

impl ClanSearchResClan {
    fn into_partial_clan(self, region: Region) -> PartialClan {
        PartialClan {
            tag: self.tag,
            color: self.hex_color,
            id: self.id,
            name: self.name,
            region,
        }
    }
}
