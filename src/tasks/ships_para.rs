use std::{error::Error, sync::Arc, time::Duration};

use parking_lot::RwLock;
use reqwest::Client;
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    structs::{ShipsPara, VortexVehicleAPIRes},
    utils::{IsacError, LoadSaveFromJson},
};

pub async fn ships_para_updater(
    client: Client,
    ships_arc: Arc<RwLock<ShipsPara>>,
    webhook_tx: UnboundedSender<String>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(86400 / 2));
    let mut last_ship_count = ships_arc.read().0.len();
    loop {
        interval.tick().await;
        let new_ships_para = match encyclopedia_vehicles(&client).await {
            Ok(new_ships_para) => new_ships_para,
            Err(err) => {
                let _ = webhook_tx.send(format!("Update ships para failed!, err: \n{err}"));
                continue;
            }
        };
        let new_ship_count = new_ships_para.0.len();
        if new_ship_count == last_ship_count {
            continue; // if the ship count is the same, skip this update
        }
        tracing::info!(
            "Ship count changed from {last_ship_count} to {new_ship_count}, updating ships para"
        );

        last_ship_count = new_ship_count;

        // save the new ships_para to json file
        new_ships_para.save_json().await;

        // update the ships_para in memory
        *ships_arc.write() = new_ships_para;

        //logging
        let _ = webhook_tx.send("ships para updated!".to_string());
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

#[expect(unused)] // not using anymore, but may use in the future
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
