use std::collections::HashMap;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use crate::utils::LoadSaveFromJson;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Dogtag(pub HashMap<u64, DogtagData>);

impl LoadSaveFromJson for Dogtag {
    const PATH: &'static str = "./web_src/dogtag.json";
}

impl Dogtag {
    const DOGTAG: Lazy<HashMap<u64, DogtagData>> = Lazy::new(|| Dogtag::load_json_sync().into());
    pub fn get(input: u64) -> Option<String> {
        if input == 0 {
            None
        } else {
            Self::DOGTAG
                .get(&input)
                .map(|f| f.icons.small.clone().unwrap_or_default())
        }
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
    title: Option<String>,
    icons: DogtagIcon,
}

#[derive(Deserialize, Serialize, Debug)]
struct DogtagIcon {
    small: Option<String>,
    large: Option<String>,
}
// exceptions:
// {
//     "id": "4290202544",
//     "name": "PCNR004",
//     "title": null,
//     "type": "border_color",
//     "color": "0x213f47",
//     "icons": {
//         "localSmall": null,
//         "small": null,
//         "large": null
//     }
// }
