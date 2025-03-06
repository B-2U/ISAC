use std::{error::Error, sync::Arc, time::Duration};

use parking_lot::RwLock;
use reqwest::Client;
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    structs::{ShipsPara, VortexVehicleAPIRes},
    utils::IsacError,
};

pub async fn ships_para_updater(
    client: Client,
    ships_arc: Arc<RwLock<ShipsPara>>,
    webhook_tx: UnboundedSender<String>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(86400 * 7));
    let mut current_version = get_game_version(&client).await.unwrap();
    loop {
        interval.tick().await;
        match get_game_version(&client).await {
            Ok(version) => {
                if version == current_version {
                    continue;
                };
                // new version, update src
                current_version = version;
                let new_ships_para = encyclopedia_vehicles(&client).await.unwrap();

                *ships_arc.write() = new_ships_para;
                //logging
            }
            Err(err) => {
                let _ = webhook_tx.send(format!("ships para updating fail!, err: \n{err}"));
            }
        }
    }
}
async fn encyclopedia_vehicles(client: &Client) -> Result<ShipsPara, IsacError> {
    client
        .get("https://vortex.worldofwarships.com/api/encyclopedia/en/vehicles/")
        .send()
        .await?
        .json::<VortexVehicleAPIRes>()
        .await
        .unwrap()
        .try_into()
}

/// check the current version of the game
async fn get_game_version(client: &Client) -> Result<String, Box<dyn Error + Send + Sync>> {
    let url = "https://vortex.worldofwarships.com/api/encyclopedia/en/";

    let res: Value = client.get(url).send().await?.json().await?;
    res["data"]["game_version"]
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| "Failed to retrieve game version".into())
}
