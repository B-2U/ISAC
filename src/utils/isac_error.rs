use std::fmt;

use crate::{
    utils::structs::{Mode, PartialClan, Region},
    Error,
};

#[derive(Debug)]
pub enum IsacError {
    Help(IsacHelp),
    Info(IsacInfo),
    Cancelled,
    UnknownError(Error),
}

impl std::error::Error for IsacError {}

impl From<IsacHelp> for IsacError {
    fn from(value: IsacHelp) -> Self {
        Self::Help(value)
    }
}

impl From<IsacInfo> for IsacError {
    fn from(value: IsacInfo) -> Self {
        Self::Info(value)
    }
}

impl From<Error> for IsacError {
    fn from(err: Error) -> Self {
        IsacError::UnknownError(err)
    }
}

impl fmt::Display for IsacError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IsacError")
    }
}

#[derive(Debug, strum::Display, thiserror::Error)]
pub enum IsacHelp {
    LackOfArguments,
}
#[derive(Debug, strum::Display, thiserror::Error)]
pub enum IsacInfo {
    UserNotLinked {
        user_name: Option<String>, // give None if its author himself
    },
    TooShortIgn {
        ign: String,
    },
    InvalidIgn {
        ign: String,
    },
    PlayerIgnNotFound {
        ign: String,
        region: Region,
    },
    PlayerHidden {
        ign: String,
    },
    PlayerNoBattle {
        ign: String,
    },
    PlayerNoBattleShip {
        ign: String,
        ship_name: String,
        mode: Mode,
    },

    InvalidClan {
        clan: String,
    },
    ClanNotFound {
        clan: String,
        region: Region,
    },
    ClanNoBattle {
        clan: PartialClan,
        season: u32,
    },

    ShipNotFound {
        ship_name: String,
    },
    APIError {
        msg: String,
    },
    GeneralError {
        msg: String,
    },
    NeedPremium {
        msg: String,
    },
    AutoCompleteError,
}

impl From<reqwest::Error> for IsacError {
    fn from(err: reqwest::Error) -> Self {
        IsacInfo::APIError {
            msg: err.to_string(),
        }
        .into()
    }
}
