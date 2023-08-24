use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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
    pub tag: String,
    pub updated_at: u64,
    pub leader_name: String,
    pub members_ids: Vec<u64>,
    pub creator_id: u64,
    pub clan_id: u64,
    // pub members: Vec<ClanDetailMember>,
    pub old_name: Option<String>,
    pub is_clan_disbanded: bool,
    pub renamed_at: Option<u64>,
    pub old_tag: Option<String>,
    pub leader_id: u64,
    pub description: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClanDetailRes {
    pub status: String,
    pub error: Option<String>,
    // meta: Meta,
    pub data: HashMap<u64, ClanDetail>, // only one in the map
}
impl ClanDetailRes {
    /// check the status is "ok" before getting the data
    pub fn data(self) -> Result<ClanDetail, IsacError> {
        if self.status.as_str() != "ok" {
            Err(IsacInfo::APIError {
                msg: self.error.unwrap_or("Unknown Error".to_string()),
            })?
        } else {
            Ok(self.data.into_values().next().unwrap())
        }
    }
}
