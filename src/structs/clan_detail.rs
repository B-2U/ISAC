use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{api, ClanTag};
use crate::utils::{IsacError, IsacInfo};

// #[derive(Debug, Deserialize, Serialize)]
// pub struct ClanDetailMember {
//     pub role: String,
//     #[serde(rename = "account_id")]
//     pub uid: u64,
//     #[serde(rename = "account_name")]
//     pub ign: String,
// }

/// mainly for the rename history, vortex takes care of all the rest
#[derive(Debug, Deserialize, Serialize)]
pub struct ClanDetail {
    pub members_count: u32,
    pub name: String,
    pub creator_name: String,
    pub created_at: u64,
    pub tag: ClanTag,
    pub updated_at: u64,
    pub leader_name: String,
    pub members_ids: Vec<u64>,
    pub creator_id: u64,
    pub clan_id: u64,
    // pub members: Vec<ClanDetailMember>,
    pub old_name: Option<String>,
    pub is_clan_disbanded: bool,
    pub renamed_at: Option<u64>,
    pub old_tag: Option<ClanTag>,
    pub leader_id: u64,
    pub description: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClanDetailAPIRes {
    #[serde(flatten)]
    status: api::Status,
    // meta: Meta,
    #[serde(default)] // its missing if an error occur
    pub data: HashMap<u64, ClanDetail>, // only one in the map
}
impl ClanDetailAPIRes {
    /// check the status is "ok" before getting the data
    pub fn data(self) -> Result<ClanDetail, IsacError> {
        if !self.status.ok() {
            Err(IsacInfo::APIError {
                msg: self.status.err_msg(),
            })?
        } else {
            Ok(self.data.into_values().next().unwrap())
        }
    }
}

#[test]
fn clan_detail_res_can_deserialize() {
    let err_json = r#"
    {
        "status": "error",
        "error": {          
            "code": 504,
            "message": "SOURCE_NOT_AVAILABLE",
            "field": null,
            "value": null
        }
    }"#;
    match serde_json::from_str::<ClanDetailAPIRes>(err_json) {
        Ok(s) => {
            if !s.status.ok() {
                println!("{}", s.status.err_msg());
            }
        }
        Err(e) => panic!("{}", e),
    };
}
