use std::collections::HashMap;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::utils::LoadSaveFromJson;

static DOGTAGS: Lazy<HashMap<u64, HashMap<String, String>>> =
    Lazy::new(|| Dogtag::load_json_sync().into());

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Dogtag(
    /// the inner hashmap is just "index": key
    pub HashMap<u64, HashMap<String, String>>,
);

impl LoadSaveFromJson for Dogtag {
    const PATH: &'static str = "./wowsinfo_data/live/shared/dogtag.json";
}

impl Dogtag {
    pub fn get(input: u64) -> Option<String> {
        if input == 0 {
            None
        } else {
            DOGTAGS
                .get(&input)
                .and_then(|inner_map| inner_map.get("index"))
                .map(|tag| format!("./wowsinfo_data/live/shared/dogtags/{tag}.png"))
        }
    }
}

impl From<Dogtag> for HashMap<u64, HashMap<String, String>> {
    fn from(value: Dogtag) -> Self {
        value.0
    }
}
