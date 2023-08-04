use crate::{
    utils::{IsacError, IsacInfo},
    Error,
};

use serde_json::Value;

pub struct Clan {
    pub tag: String,
    pub color: String,
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

impl Clan {
    /// parsing clan from json
    pub fn parse(json: Value) -> Result<Clan, IsacError> {
        Self::_parse(json).map_err(|e| match e.downcast::<IsacError>() {
            Ok(isac) => *isac,
            Err(err) => IsacError::UnknownError(err),
        })
    }
    fn _parse(json: Value) -> Result<Clan, Error> {
        let "ok" = json.get("status").and_then(|f|f.as_str()).unwrap() else {
            let err_msg = json.get("error").and_then(|f| f.as_str());
            match err_msg {
                Some(err) => Err(IsacInfo::APIError { msg:err.to_string() })?,
                None => Err(IsacInfo::GeneralError { msg: "parsing player's clan failed".to_string() })?
            }
        };
        let sec_layer = json.get("data").unwrap();

        let clan_id = sec_layer.get("clan_id").unwrap();
        // not in a clan
        let clan_id = match clan_id.is_u64() {
            true => clan_id.as_u64().unwrap(),
            false => return Ok(Clan::default()),
        };

        let third_layer = sec_layer.get("clan").unwrap();
        let name = third_layer.get("name").and_then(|f| f.as_str()).unwrap();
        let tag = third_layer.get("tag").and_then(|f| f.as_str()).unwrap();
        let color = third_layer.get("color").and_then(|f| f.as_u64()).unwrap();
        // let members_count = third_layer
        //     .get("members_count")
        //     .and_then(|f| f.as_u64())
        //     .unwrap();

        Ok(Clan {
            tag: tag.to_string(),
            color: Self::_decimal_to_hex(color),
            id: clan_id,
            name: name.to_string(),
        })
    }
    fn _decimal_to_hex(input: u64) -> String {
        format!("{:x}", input)
    }
}
