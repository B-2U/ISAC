use bytes::Bytes;

use reqwest::Client;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::utils::{
    structs::{Clan, Player, ShipClass, ShipTier, Statistic},
    IsacError, IsacInfo,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct OverallData {
    pub div: OverallDataDiv,
    pub tier: OverallDataTier,
    pub class: OverallDataClass,
    pub clan: Clan,
    pub user: Player,
}
impl OverallData {
    // TODO turn render() into a trait for all templates?
    // pub fn render(&self) -> String {
    //     let mut reg = Handlebars::new();
    //     reg.register_template_file("overall", TEMPLATE_OVERALL)
    //         .unwrap();
    //     // let mut output_file = std::fs::File::create("test.html").unwrap();
    //     if cfg!(windows) {
    //         let file = std::fs::File::create(TEMPLATE_OVERALL_JSON).expect("Failed to create file");
    //         serde_json::to_writer(file, self).expect("Failed to write JSON to file");
    //     };
    //     reg.render("overall", self).unwrap()
    // }
    // TODO 每個render只有url不一樣 該怎麼抽象他們?
    pub async fn render(&self, client: &Client) -> Result<Bytes, IsacError> {
        Ok(client
            .post("http://localhost:3000/overall")
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
    pub async fn render_tiers(&self, client: &Client) -> Result<Bytes, IsacError> {
        Ok(client
            .post("http://localhost:3000/overall_tiers")
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
pub struct OverallDataDiv {
    pub pvp: Statistic,
    pub pvp_solo: Statistic,
    pub pvp_div2: Statistic,
    pub pvp_div3: Statistic,
}
impl OverallDataDiv {
    pub fn new(
        pvp: Statistic,
        pvp_solo: Statistic,
        pvp_div2: Statistic,
        pvp_div3: Statistic,
    ) -> Self {
        Self {
            pvp,
            pvp_solo,
            pvp_div2,
            pvp_div3,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OverallDataTier {
    #[serde(rename = "1")]
    pub one: Statistic,
    #[serde(rename = "2")]
    pub two: Statistic,
    #[serde(rename = "3")]
    pub three: Statistic,
    #[serde(rename = "4")]
    pub four: Statistic,
    #[serde(rename = "5")]
    pub five: Statistic,
    #[serde(rename = "6")]
    pub six: Statistic,
    #[serde(rename = "7")]
    pub seven: Statistic,
    #[serde(rename = "8")]
    pub eight: Statistic,
    #[serde(rename = "9")]
    pub nine: Statistic,
    #[serde(rename = "10")]
    pub ten: Statistic,
    #[serde(rename = "11")]
    pub eleven: Statistic,
}

impl From<HashMap<ShipTier, Statistic>> for OverallDataTier {
    // TODO, related to the sort_tier return type problem
    fn from(mut value: HashMap<ShipTier, Statistic>) -> Self {
        Self {
            one: value.remove(&ShipTier::I).unwrap(),
            two: value.remove(&ShipTier::II).unwrap(),
            three: value.remove(&ShipTier::III).unwrap(),
            four: value.remove(&ShipTier::IV).unwrap(),
            five: value.remove(&ShipTier::V).unwrap(),
            six: value.remove(&ShipTier::VI).unwrap(),
            seven: value.remove(&ShipTier::VII).unwrap(),
            eight: value.remove(&ShipTier::VIII).unwrap(),
            nine: value.remove(&ShipTier::IX).unwrap(),
            ten: value.remove(&ShipTier::X).unwrap(),
            eleven: value.remove(&ShipTier::XI).unwrap(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OverallDataClass {
    pub ss: Statistic,
    pub dd: Statistic,
    pub ca: Statistic,
    pub bb: Statistic,
    pub cv: Statistic,
}

// impl OverallDataClass {
//     fn new(dd: Statistic, ca: Statistic, bb: Statistic, cv: Statistic, ss: Statistic) -> Self {
//         Self { dd, ca, bb, cv, ss }
//     }
// }

impl From<HashMap<ShipClass, Statistic>> for OverallDataClass {
    // TODO, related to the sort_class return type problem
    fn from(mut value: HashMap<ShipClass, Statistic>) -> Self {
        Self {
            ss: value.remove(&ShipClass::SS).unwrap(),
            dd: value.remove(&ShipClass::DD).unwrap(),
            ca: value.remove(&ShipClass::CA).unwrap(),
            bb: value.remove(&ShipClass::BB).unwrap(),
            cv: value.remove(&ShipClass::CV).unwrap(),
        }
    }
}
