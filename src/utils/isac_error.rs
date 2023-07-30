use std::fmt;

use crate::Error;

use super::user::Region;

#[derive(Debug)]
pub enum IsacError {
    LackOfArguments,
    UserNotLinked { msg: String },
    TooShortIgn { ign: String },
    APIError { msg: String },
    InvalidIgn { ign: String },
    PlayerIgnNotFound { ign: String, region: Region },
    PlayerHidden { ign: String },
    PlayerNoBattle { ign: String },
    Cancelled,
    UnkownError(Error),
}

impl std::error::Error for IsacError {}

impl fmt::Display for IsacError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self, f)
    }
}
