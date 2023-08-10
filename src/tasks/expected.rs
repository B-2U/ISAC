use std::{fs::File, sync::Arc, time::Duration};

use parking_lot::RwLock;
use reqwest::Client;
use serde_json::to_writer;
use tracing::log::warn;

use crate::utils::structs::{ExpectedJs, EXPECTED_JS_PATH};

pub async fn expected_updater(client: Client, expected_arc: Arc<RwLock<ExpectedJs>>) {
    let mut interval = tokio::time::interval(Duration::from_secs(86400));
    loop {
        interval.tick().await;
        if let Ok(expected_js) = request(&client).await {
            let file = File::create(EXPECTED_JS_PATH).expect("Failed to create file");
            to_writer(file, &expected_js).expect("Failed to write JSON to file");
            *expected_arc.write() = expected_js;
        } else {
            warn!("expected js updating fail!");
        }
    }
}

async fn request(client: &Client) -> Result<ExpectedJs, reqwest::Error> {
    client
        .get("https://api.wows-numbers.com/personal/rating/expected/json/")
        .send()
        .await?
        .json::<ExpectedJs>()
        .await
}
