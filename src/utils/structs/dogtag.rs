use std::collections::HashMap;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::utils::LoadSaveFromJson;

static DOGTAGS: Lazy<HashMap<u64, String>> = Lazy::new(|| Dogtag::load_json_sync().into());

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Dogtag(pub HashMap<u64, String>);

impl LoadSaveFromJson for Dogtag {
    const PATH: &'static str = "./web_src/dogtag.json";
}

impl Dogtag {
    pub fn get(input: u64) -> Option<String> {
        if input == 0 {
            None
        } else {
            DOGTAGS
                .get(&input)
                .map(|str| format!("./web_src/dogtags/{str}.png"))
        }
    }
}

impl From<Dogtag> for HashMap<u64, String> {
    fn from(value: Dogtag) -> Self {
        value.0
    }
}
