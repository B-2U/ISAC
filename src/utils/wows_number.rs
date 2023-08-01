use reqwest::Url;

use super::{
    user::{PartialPlayer, Player},
    IsacError,
};

pub trait WowsNumber {
    /// get the player wows number url
    fn wows_number(&self) -> Result<Url, IsacError>;
}

impl WowsNumber for PartialPlayer {
    fn wows_number(&self) -> Result<Url, IsacError> {
        self.region.number_url(format!("/player/{},/", self.uid))
    }
}

impl WowsNumber for Player {
    fn wows_number(&self) -> Result<Url, IsacError> {
        self.region.number_url(format!("/player/{},/", self.uid))
    }
}
