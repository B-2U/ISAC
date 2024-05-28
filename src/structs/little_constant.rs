use crate::utils::LoadSaveFromJson;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct LittleConstant {
    pub clan_season: u32,
}

impl Default for LittleConstant {
    fn default() -> Self {
        Self { clan_season: 22 } // need a better way to update this...
    }
}

impl LoadSaveFromJson for LittleConstant {
    const PATH: &'static str = "./web_src/little_constant.json";
}
