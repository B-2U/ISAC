use std::fmt;

use crate::Error;

use super::user::Region;

#[derive(Debug)]
pub enum IsacError {
    Help(IsacHelp),
    Info(IsacInfo),
    Cancelled,
    UnkownError(Error),
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
    PlayerIgnNotFound { ign: String, region: Region },
    PlayerHidden { ign: String },
    PlayerNoBattle { ign: String },
}

impl std::error::Error for IsacInfo {}

impl fmt::Display for IsacInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self, f)
    }
}
