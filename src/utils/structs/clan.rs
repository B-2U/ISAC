use crate::utils::IsacInfo;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug)]
pub struct Clan {
    pub tag: String,   // e.g. PANTS, do not include [ ]
    pub color: String, // hex color string
    pub id: u64,
    pub name: String,
}

/// should be only for those player not in a clan
impl Default for Clan {
    fn default() -> Self {
        Self {
            tag: "".to_string(),
            color: "#fff".to_string(),
            id: 0,
            name: "".to_string(),
        }
    }
}

impl TryFrom<Value> for Clan {
    type Error = IsacInfo;

    fn try_from(json: Value) -> Result<Self, Self::Error> {
        fn err(s: impl AsRef<str>) -> IsacInfo {
            IsacInfo::APIError {
                msg: s.as_ref().into(),
            }
        }
        let "ok" = json.get("status").and_then(|f|f.as_str()).unwrap() else {
            let err_msg = json.get("error").and_then(|f| f.as_str());
            match err_msg {
                Some(err) => Err(IsacInfo::APIError { msg:err.to_string() })?,
                None => Err(IsacInfo::GeneralError { msg: "parsing player's clan failed".to_string() })?
            }
        };
        let sec_layer = json.get("data").ok_or(err("no data"))?;

        let clan_id = sec_layer.get("clan_id").ok_or(err("no clan_id"))?;
        // not in a clan
        let clan_id = match clan_id.is_u64() {
            true => clan_id.as_u64().ok_or(err("clan_id convert failed"))?,
            false => return Ok(Clan::default()),
        };

        let third_layer = sec_layer.get("clan").unwrap();
        let name = third_layer
            .get("name")
            .and_then(|f| f.as_str())
            .ok_or(err("no name"))?;
        let tag = third_layer
            .get("tag")
            .and_then(|f| f.as_str())
            .ok_or(err("no tag"))?;
        let color = third_layer
            .get("color")
            .and_then(|f| f.as_u64())
            .ok_or(err("no color"))?;
        // let members_count = third_layer
        //     .get("members_count")
        //     .and_then(|f| f.as_u64())
        //     .unwrap();

        Ok(Clan {
            tag: tag.to_string(),
            color: Self::decimal_to_hex(color),
            id: clan_id,
            name: name.to_string(),
        })
    }
}

impl Clan {
    pub fn decimal_to_hex(input: u64) -> String {
        format!("{:x}", input)
    }
}
