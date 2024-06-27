use crate::utils::LoadSaveFromJson;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct LittleConstant {
    pub clan_season: u32,
}

impl Default for LittleConstant {
    fn default() -> Self {
        Self { clan_season: 5 } // will be updated in first .clan command
    }
}

impl LoadSaveFromJson for LittleConstant {
    const PATH: &'static str = "./web_src/little_constant.json";
}
