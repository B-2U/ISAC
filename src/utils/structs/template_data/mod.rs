mod overall;
use bytes::Bytes;
pub use overall::*;

mod single_ship;
use poise::async_trait;
use reqwest::Client;
pub use single_ship::*;

use crate::utils::{IsacError, IsacInfo};

#[async_trait]
pub trait Render {
    const RENDER_URL: &'static str; // Associated constant for the URL
    async fn render(&self, client: &Client) -> Result<Bytes, IsacError>
    where
        Self: serde::Serialize,
    {
        Ok(client
            .post(format!("http://localhost:3000/{}", Self::RENDER_URL))
            .json(&self)
            .send()
            .await
            .map_err(|_| IsacInfo::GeneralError {
                msg: "screenshot failed".to_string(),
            })?
            .bytes()
            .await
            .map_err(|_| IsacInfo::GeneralError {
                msg: "screenshot failed".to_string(),
            })?)
    }
}
