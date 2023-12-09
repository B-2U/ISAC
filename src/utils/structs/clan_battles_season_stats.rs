// the struct for player's clan battles seasons stats
// https://api.worldofwarships.asia/wows/clans/seasonstats/

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::utils::{structs::api, IsacError, IsacInfo};

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerClanBattleAPIRes {
    #[serde(flatten)]
    pub status: api::Status,
    pub data: HashMap<u64, PlayerClanBattle>, // only one in the map
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerClanBattle {
    pub seasons: Vec<PlayerClanBattleSeason>,
}

impl TryFrom<PlayerClanBattleAPIRes> for PlayerClanBattle {
    type Error = IsacError;

    fn try_from(value: PlayerClanBattleAPIRes) -> Result<Self, Self::Error> {
        if !value.status.ok() {
            return Err(IsacInfo::APIError {
                msg: value.status.err_msg(),
            }
            .into());
        }
        Ok(value
            .data
            .into_iter()
            .next()
            .map(|s| s.1)
            .unwrap_or(PlayerClanBattle { seasons: vec![] }))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerClanBattleSeason {
    pub season_id: u32,
    pub wins: u64,
    pub battles: u64,
    pub damage_dealt: u64,
    pub art_agro: u64,
    pub frags: u64,
    pub damage_scouting: u64,
}
// {
//     "status": "ok",
//     "meta": {
//       "count": 1,
//       "hidden": null
//     },
//     "data": {
//       "2025455227": {
//         "seasons": [
//           {"season_id": 11, "wins": 98, "battles": 155},
//           {"season_id": 13, "wins": 23, "battles": 44},
//           {"season_id": 15, "wins": 32, "battles": 55},
//           {"season_id": 21, "wins": 56, "battles": 82},
//           {"season_id": 17, "wins": 46, "battles": 60},
//           {"season_id": 16, "wins": 27, "battles": 52},
//           {"season_id": 19, "wins": 46, "battles": 70},
//           {"season_id": 18, "wins": 78, "battles": 127},
//           {"season_id": 22, "wins": 50, "battles": 76},
//           {"season_id": 213, "wins": 19, "battles": 23},
//           {"season_id": 212, "wins": 74, "battles": 92},
//           {"season_id": 20, "wins": 59, "battles": 104},
//           {"season_id": 23, "wins": 18, "battles": 22}
//         ]
//       }
//     }
//   }
