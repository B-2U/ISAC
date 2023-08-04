use std::fmt;

use crate::{utils::structs::Region, Error};

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

impl fmt::Display for IsacError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self, f)
    }
}

#[derive(Debug)]
pub enum IsacHelp {
    LackOfArguments,
}
impl std::error::Error for IsacHelp {}

impl fmt::Display for IsacHelp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self, f)
    }
}

#[derive(Debug)]
pub enum IsacInfo {
    UserNotLinked { msg: String },
    TooShortIgn { ign: String },
    APIError { msg: String },
    InvalidIgn { ign: String },
    InvalidClan { clan: String },
    PlayerIgnNotFound { ign: String, region: Region },
    PlayerHidden { ign: String },
    PlayerNoBattle { ign: String },
    ClanNotFound { clan: String, region: Region },
    GeneralError { msg: String },
}

impl std::error::Error for IsacInfo {}

impl fmt::Display for IsacInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self, f)
    }
}
