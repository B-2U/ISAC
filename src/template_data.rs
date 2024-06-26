use bytes::Bytes;
use reqwest::Client;

mod overall;
pub use overall::*;
mod single_ship;
pub use single_ship::*;
mod leaderboard;
pub use leaderboard::*;
mod clan;
pub use clan::*;
mod clan_season;
pub use clan_season::*;
mod recent;
pub use recent::*;

mod overall_cw;
pub use overall_cw::*;

mod server_top;
pub use server_top::*;

use crate::utils::{IsacError, IsacInfo};

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
