use bytes::Bytes;

use reqwest::Client;

use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};

use crate::utils::{
    structs::{Clan, Player, Ship, Statistic},
    IsacError, IsacInfo,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct SingleShipData {
    pub ship: Ship,
    pub main_mode: Statistic,
    #[serde(serialize_with = "serialize_sub_modes")]
    pub sub_modes: Option<SingleShipDataSub>,
    pub clan: Clan,
    pub user: Player,
}
fn serialize_sub_modes<S>(
    sub_modes: &Option<SingleShipDataSub>, // Replace with your actual types
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match sub_modes {
        Some(sub_modes) => sub_modes.serialize(serializer),
        None => serializer.serialize_struct("SingleShipDataSub", 0)?.end(),
    }
}
impl SingleShipData {
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
pub struct SingleShipDataSub {
    pub pvp_solo: Statistic,
    pub pvp_div2: Statistic,
    pub pvp_div3: Statistic,
}
impl SingleShipDataSub {
    pub fn new(pvp_solo: Statistic, pvp_div2: Statistic, pvp_div3: Statistic) -> Self {
        Self {
            pvp_solo,
            pvp_div2,
            pvp_div3,
        }
    }
}
