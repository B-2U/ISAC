use poise::async_trait;
use reqwest::Url;

use crate::Context;

use super::{
    structs::{Clan, PartialPlayer, Player},
    wws_api::WowsApi,
    IsacError,
};

// todo: 這文件和trait應該怎麼命名, 放哪?
#[async_trait]
pub trait PlayerCommon {
    /// get the player wows number url
    fn wows_number(&self) -> Result<Url, IsacError>;

    /// get the clan info, return a default clan if the player isn't belong to any clan
    async fn clan(&self, ctx: &Context<'_>) -> Result<Clan, IsacError>;
}

#[async_trait]
impl PlayerCommon for PartialPlayer {
    fn wows_number(&self) -> Result<Url, IsacError> {
        self.region.number_url(format!("/player/{},/", self.uid))
    }

    async fn clan(&self, ctx: &Context<'_>) -> Result<Clan, IsacError> {
        let api = WowsApi::new(&ctx);
        api.player_clan(&self.region, self.uid).await
    }
}

#[async_trait]
impl PlayerCommon for Player {
    fn wows_number(&self) -> Result<Url, IsacError> {
        self.region.number_url(format!("/player/{},/", self.uid))
    }
    async fn clan(&self, ctx: &Context<'_>) -> Result<Clan, IsacError> {
        let api = WowsApi::new(&ctx);
        api.player_clan(&self.region, self.uid).await
    }
}
