use std::{sync::Arc, time::Duration};

use parking_lot::RwLock;
use reqwest::Client;
use tokio::sync::mpsc::UnboundedSender;

use crate::{LoadSaveFromJson, structs::ExpectedJs};

pub async fn expected_updater(
    client: Client,
    expected_arc: Arc<RwLock<ExpectedJs>>,
    webhook_tx: UnboundedSender<String>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(86400));
    loop {
        interval.tick().await;
        match request(&client).await {
            Ok(expected_js) => {
                expected_js.save_json().await;
                *expected_arc.write() = expected_js;
            }
            Err(err) => {
                let _ = webhook_tx.send(format!("expected js updating fail!, err: \n{err}"));
            }
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
