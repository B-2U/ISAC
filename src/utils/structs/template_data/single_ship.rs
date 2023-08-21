use bytes::Bytes;

use reqwest::Client;

use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};

use super::Render;
use crate::utils::{
    structs::{PartialClan, Player, Ship, Statistic},
    IsacError, IsacInfo,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct SingleShipTemplate {
    pub ship: Ship,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranking: Option<u64>,
    pub main_mode_name: String,
    pub main_mode: Statistic,
    #[serde(serialize_with = "serialize_sub_modes")]
    pub sub_modes: Option<SingleShipTemplateSub>,
    pub clan: PartialClan,
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
