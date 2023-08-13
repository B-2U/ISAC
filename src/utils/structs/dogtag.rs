use std::collections::HashMap;

use once_cell::sync::Lazy;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use crate::utils::LoadFromJson;

#[derive(Deserialize, Debug)]
pub struct Dogtag(pub HashMap<u64, DogtagData>);

impl Dogtag {
    const DOGTAG: Lazy<HashMap<u64, DogtagData>> = Lazy::new(|| {
        Dogtag::load_json_sync("./web_src/dogtag.json")
            .unwrap()
            .into()
    });
    pub fn get(input: Option<u64>) -> Option<String> {
        let Some(input) = input else {
            return None;
        };
        Self::DOGTAG.get(&input).map(|f| f.icons.small.to_string())
    }
}

impl From<Dogtag> for HashMap<u64, DogtagData> {
    fn from(value: Dogtag) -> Self {
        value.0
    }
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct DogtagData {
    #[serde_as(as = "DisplayFromStr")]
    id: u64,
    title: String,
    icons: DogtagIcon,
}

#[derive(Deserialize, Serialize, Debug)]
struct DogtagIcon {
    small: Url,
    large: Url,
}