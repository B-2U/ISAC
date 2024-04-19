use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::AsyncWriteExt;

use crate::utils::structs::{PartialPlayer, ShipStatsCollection};

#[derive(Debug, Serialize, Deserialize)]
pub struct RecentPlayer {
    #[serde(default, skip_serializing)]
    pub player: PartialPlayer,
    pub last_update_at: u64, // unix timestamp
    #[serde(deserialize_with = "last_request_migrate")]
    pub last_request: RecentPlayerType,
    pub data: BTreeMap<u64, ShipStatsCollection>,
}

fn last_request_migrate<'de, D>(deserializer: D) -> Result<RecentPlayerType, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v: Value = Deserialize::deserialize(deserializer)?;
    Ok(
        if let Ok(last_request) = serde_json::from_value::<RecentPlayerType>(v.clone()) {
            last_request
        } else {
            RecentPlayerType::Normal(v.as_u64().unwrap_or_default())
        },
    )
}

impl RecentPlayer {
    /// try to get the given date data first, then find the closest one, None if no matched
    pub async fn get_date(&self, timestamp: &u64) -> Option<(u64, ShipStatsCollection)> {
        if let Some((k, v)) = self.data.get_key_value(timestamp) {
            Some((*k, v.clone()))
        } else {
            self.data
                .iter()
                .find(|(&date, _)| date >= *timestamp)
                .map(|(date, data)| (*date, data.clone()))
        }
    }

    /// return the timestamp keys which earlier than the given one, this is for making choices for user
    pub fn available_dates(&self, timestamp: &u64) -> Vec<u64> {
        self.data
            .iter()
            .filter(|(&date, _)| date < *timestamp)
            .map(|(&date, _)| date)
            .collect()
    }
    /// load the player's recent data, return None if he is not inside
    pub async fn load(player: &PartialPlayer) -> Option<Self> {
        let path = Self::get_path(&player);
        if let Ok(file) = std::fs::File::open(&path) {
            let mut data: RecentPlayer = tokio::task::spawn_blocking(move || {
                let reader = std::io::BufReader::new(file);
                serde_json::from_reader(reader)
            })
            .await
            .unwrap()
            .unwrap_or_else(|err| panic!("Failed to deserialize file: {:?}\n Err: {err}", path,));
            data.player = *player;
            Some(data)
        } else {
            None
        }
    }

    /// init new player file
    pub async fn init(player: &PartialPlayer) -> Self {
        Self {
            player: *player,
            last_update_at: 0,
            last_request: RecentPlayerType::Normal(0),
            data: Default::default(),
        }
    }

    /// save player data
    pub async fn save(&self) {
        let path = Self::get_path(&self.player);

        let mut file = tokio::fs::File::create(&path)
            .await
            .unwrap_or_else(|err| panic!("failed to create file: {:?}, Err: {err}", path));
        let json_bytes = serde_json::to_vec(&self).unwrap_or_else(|err| {
            panic!(
                "Failed to serialize struct: {:?} to JSON. Err: {err}",
                std::any::type_name::<Self>(),
            )
        });
        if let Err(err) = file.write_all(&json_bytes).await {
            panic!("Failed to write JSON to file: {:?}. Err: {err}", path,);
        }
    }

    /// get player's file path
    fn get_path(player: &PartialPlayer) -> String {
        format!(
            "./recent_DB/players/{}/{}.json",
            player.region.lower(),
            player.uid
        )
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum RecentPlayerType {
    #[serde(alias = "prime")]
    Premium,
    Normal(u64),
}

// #[derive(Debug, Serialize, Deserialize)]
// pub struct RecentPlayerData { }
