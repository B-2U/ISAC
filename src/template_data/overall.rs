use bytes::Bytes;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::Render;
use crate::{
    renderer::Renderer,
    structs::{PartialClan, Player, ShipClass, ShipTier, Statistic},
    utils::{IsacError, IsacInfo},
};

#[derive(Serialize, Deserialize, Debug)]
pub struct OverallTemplate {
    pub div: OverallTemplateDiv,
    pub tier: OverallTemplateTier,
    pub class: OverallTemplateClass,
    pub clan: Option<PartialClan>,
    pub user: Player,
}

impl Render for OverallTemplate {
    const RENDER_URL: &'static str = "overall";
}

impl OverallTemplate {
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
    // QA trait化了, 但是這個struct同時要兩個版本 該怎麼寫?
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
    pub async fn render_test(&self, renderer: &Renderer) -> Result<Bytes, IsacError> {
        renderer.render("overall", &self).await
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OverallTemplateDiv {
    pub pvp: Statistic,
    pub pvp_solo: Statistic,
    pub pvp_div2: Statistic,
    pub pvp_div3: Statistic,
}
impl OverallTemplateDiv {
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
pub struct OverallTemplateTier {
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

impl From<HashMap<ShipTier, Statistic>> for OverallTemplateTier {
    fn from(mut value: HashMap<ShipTier, Statistic>) -> Self {
        Self {
            one: value.remove(&ShipTier::I).unwrap_or_default(),
            two: value.remove(&ShipTier::II).unwrap_or_default(),
            three: value.remove(&ShipTier::III).unwrap_or_default(),
            four: value.remove(&ShipTier::IV).unwrap_or_default(),
            five: value.remove(&ShipTier::V).unwrap_or_default(),
            six: value.remove(&ShipTier::VI).unwrap_or_default(),
            seven: value.remove(&ShipTier::VII).unwrap_or_default(),
            eight: value.remove(&ShipTier::VIII).unwrap_or_default(),
            nine: value.remove(&ShipTier::IX).unwrap_or_default(),
            ten: value.remove(&ShipTier::X).unwrap_or_default(),
            eleven: value.remove(&ShipTier::XI).unwrap_or_default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OverallTemplateClass {
    pub ss: Statistic,
    pub dd: Statistic,
    pub ca: Statistic,
    pub bb: Statistic,
    pub cv: Statistic,
}

// impl OverallTemplateClass {
//     fn new(dd: Statistic, ca: Statistic, bb: Statistic, cv: Statistic, ss: Statistic) -> Self {
//         Self { dd, ca, bb, cv, ss }
//     }
// }

impl From<HashMap<ShipClass, Statistic>> for OverallTemplateClass {
    fn from(mut value: HashMap<ShipClass, Statistic>) -> Self {
        Self {
            ss: value.remove(&ShipClass::SS).unwrap_or_default(),
            dd: value.remove(&ShipClass::DD).unwrap_or_default(),
            ca: value.remove(&ShipClass::CA).unwrap_or_default(),
            bb: value.remove(&ShipClass::BB).unwrap_or_default(),
            cv: value.remove(&ShipClass::CV).unwrap_or_default(),
        }
    }
}
