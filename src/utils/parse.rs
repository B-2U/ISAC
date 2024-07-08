use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    structs::Region,
    utils::{IsacError, IsacInfo},
};

/// parsing region and ign from str, for example: `[ASIA] B2U` or `ASIA B2U`
/// # Error
/// [`IsacInfo::GeneralError`] if received a malformed input
pub fn parse_region_ign(input: &str) -> Result<(Region, String), IsacError> {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[?(\w+)\]?\s+(\w+)").unwrap());
    let (_, [region_str, ign]) = RE
        .captures(input)
        .ok_or(IsacInfo::GeneralError {
            msg: "invalid input!".to_string(),
        })?
        .extract();
    let region = Region::parse(region_str).ok_or(IsacInfo::GeneralError {
        msg: format!("`{region_str}` is not a valid region"),
    })?;
    Ok((region, ign.to_string()))
}
