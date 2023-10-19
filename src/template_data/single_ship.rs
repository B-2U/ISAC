use bytes::Bytes;

use reqwest::Client;

use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};

use super::Render;
use crate::{
    utils::{
        structs::{Mode, PartialClan, Player, Ship, ShipId, ShipModeStatsPair, Statistic},
        IsacError, IsacInfo,
    },
    Context,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct SingleShipTemplate {
    pub ship: Ship,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranking: Option<u64>,
    pub suffix: String, // additional info, e.g. (last 2 days) (Rank)
    pub main_mode: Statistic,
    #[serde(serialize_with = "serialize_sub_modes")]
    pub sub_modes: Option<SingleShipTemplateSub>,
    pub clan: Option<PartialClan>,
    pub user: Player,
}
impl Render for SingleShipTemplate {
    const RENDER_URL: &'static str = "single_ship";
}

fn serialize_sub_modes<S>(
    sub_modes: &Option<SingleShipTemplateSub>, // Replace with your actual types
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match sub_modes {
        Some(sub_modes) => sub_modes.serialize(serializer),
        None => serializer
            .serialize_struct("SingleShipTemplateSub", 0)?
            .end(),
    }
}
impl SingleShipTemplate {
    pub async fn render(&self, client: &Client) -> Result<Bytes, IsacError> {
        Ok(client
            .post("http://localhost:3000/single_ship")
            .json(&self)
            .send()
            .await
            .map_err(|_| IsacInfo::GeneralError {
                msg: "screenshot failed".to_string(),
            })?
            .bytes()
            .await
            .map_err(|_| IsacInfo::GeneralError {
                msg: "screenshot failed".to_string(),
            })?)
    }
    // QA 這種方式真的算正面嗎?
    /// a helper function to build up the structure, raise [`IsacInfo::PlayerNoBattleShip`] if the main_mode battle_counts is 0
    pub fn new(
        ctx: &Context<'_>,
        ship: Ship,
        ranking: Option<u64>,
        suffix: String,
        ship_id: ShipId,
        ship_stats: ShipModeStatsPair,
        mode: Mode,
        clan: Option<PartialClan>,
        player: Player,
    ) -> Result<Self, IsacError> {
        let Some(main_mode) = ship_stats.to_statistic(&ship_id, &ctx.data().expected_js, mode)
        else {
            Err(IsacInfo::PlayerNoBattleShip {
                ign: player.ign.clone(),
                ship_name: ship.name.clone(),
                mode,
            })?
        };
        let sub_modes = if let Mode::Rank = mode {
            None
        } else {
            Some(SingleShipTemplateSub::new(
                ship_stats
                    .to_statistic(&ship_id, &ctx.data().expected_js, Mode::Solo)
                    .unwrap_or_default(),
                ship_stats
                    .to_statistic(&ship_id, &ctx.data().expected_js, Mode::Div2)
                    .unwrap_or_default(),
                ship_stats
                    .to_statistic(&ship_id, &ctx.data().expected_js, Mode::Div3)
                    .unwrap_or_default(),
            ))
        };
        Ok(SingleShipTemplate {
            ship,
            ranking,
            suffix,
            main_mode,
            sub_modes,
            clan,
            user: player,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SingleShipTemplateSub {
    pub pvp_solo: Statistic,
    pub pvp_div2: Statistic,
    pub pvp_div3: Statistic,
}
impl SingleShipTemplateSub {
    pub fn new(pvp_solo: Statistic, pvp_div2: Statistic, pvp_div3: Statistic) -> Self {
        Self {
            pvp_solo,
            pvp_div2,
            pvp_div3,
        }
    }
}
