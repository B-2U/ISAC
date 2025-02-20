use std::{
    collections::BTreeMap,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::structs::{PartialPlayer, ShipStatsCollection};

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerSnapshots {
    // PartialPlayer will get in Self::load()
    #[serde(default, skip_serializing)]
    pub player: PartialPlayer,
    pub last_update_at: u64, // unix timestamp
    pub last_request: PlayerSnapshotsType,
    pub data: BTreeMap<u64, ShipStatsCollection>,
}

impl PlayerSnapshots {
    /// try to get the given date data first, then find the closest one, None if no matched
    pub async fn get_date(&self, timestamp: &u64) -> Option<(u64, ShipStatsCollection)> {
        if let Some((k, v)) = self.data.get_key_value(timestamp) {
            Some((*k, v.clone()))
        } else {
            self.data
                .iter()
                .find(|&(date, _)| date >= timestamp)
                .map(|(date, data)| (*date, data.clone()))
        }
    }

    /// get the latest snapshot, it should always be Some()
    pub fn latest_snapshot(&self) -> Option<ShipStatsCollection> {
        self.data.last_key_value().map(|(_, v)| v.clone())
    }

    /// return the timestamp keys which earlier than the given one, this is for making choices for user
    pub fn available_dates(&self, timestamp: &u64) -> Vec<u64> {
        self.data
            .iter()
            .filter(|&(date, _)| date < timestamp)
            .map(|(&date, _)| date)
            .collect()
    }

    /// update the Self.last_request to now
    pub fn update_last_request(&mut self, is_premium: bool) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_request = if is_premium {
            PlayerSnapshotsType::Premium
        } else {
            PlayerSnapshotsType::Normal(now)
        };
    }

    /// load the player's recent data, return None if he is not inside
    pub async fn load(player: PartialPlayer) -> Option<Self> {
        let path = Self::get_path(&player);
        // std::fs::File::open() is as fast as path.exists()
        if let Ok(file) = std::fs::File::open(&path) {
            let mut data: PlayerSnapshots = tokio::task::spawn_blocking(move || {
                let json_str = std::io::read_to_string(file).unwrap();
                serde_json::from_str(&json_str)
            })
            .await
            .unwrap()
            .unwrap_or_else(|err| panic!("Failed to deserialize file: {:?}\n Err: {err}", path,));
            data.player = player;
            Some(data)
        } else {
            None
        }
    }

    /// add the given record into the snapshot, and update the `last_updated_at`
    pub fn insert(&mut self, ships: ShipStatsCollection) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.data.insert(now, ships);
        self.last_update_at = now;
    }

    /// init new player file
    pub async fn init(player: PartialPlayer) -> Self {
        Self {
            player,
            last_update_at: 0,
            last_request: PlayerSnapshotsType::Normal(0),
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
    fn get_path(player: &PartialPlayer) -> PathBuf {
        let mut path = PathBuf::from("./recent_DB/players/");
        path.push(player.region.lower());
        path.push(format!("{}.json", player.uid));
        path
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum PlayerSnapshotsType {
    #[serde(alias = "prime")]
    Premium,
    Normal(u64),
}

// #[derive(Debug, Serialize, Deserialize)]
// pub struct PlayerSnapshotsData { }
