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
    let mut interval = tokio::time::interval(Duration::from_secs(86400 / 2));
    let mut current_version = get_game_version(&client).await.unwrap_or_default();
    loop {
        interval.tick().await;
        match get_game_version(&client).await {
            Ok(version) => {
                if version == current_version {
                    continue;
                };
                // new version, update src
                let Ok(new_ships_para) = encyclopedia_vehicles(&client).await else {
                    continue;
                };
                current_version = version;

                *ships_arc.write() = new_ships_para;
                //logging
                let _ =
                    webhook_tx.send(format!("ships para updated to version {current_version}!"));
            }
            Err(err) => {
                let _ = webhook_tx.send(format!("get_game_version fail!, err: \n{err}"));
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
    let Some(data) = res.get("data") else {
        return Err(r#"["data"] not found in encyclopedia"#.into());
    };
    let Some(game_version) = data.get("game_version") else {
        return Err(r#"["data"]["game_version"] not found in encyclopedia"#.into());
    };
    game_version
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| r#"["data"]["game_version"] is not a string"#.into())
}
