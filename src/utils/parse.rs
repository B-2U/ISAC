use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    dc_utils::autocomplete::AutocompleteClan,
    structs::{ClanTag, Region},
    utils::{IsacError, IsacInfo},
};

// parsing region and clan tag from str, for example: `[PANTS] Dont Cap Kill All (ASIA)`
/// # Error
/// [`IsacInfo::GeneralError`] if received a malformed input
pub fn parse_region_clan(input: &str) -> Result<AutocompleteClan, IsacError> {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[([\w|-]+)].+\((\w+)\)").unwrap());
    let (_, [clan_tag, region_str]) = RE
        .captures(input)
        .ok_or(IsacInfo::GeneralError {
            msg: "invalid input!".to_string(),
        })?
        .extract();
    let region = Region::parse(region_str).ok_or(IsacInfo::GeneralError {
        msg: format!("`{region_str}` is not a valid region"),
    })?;
    Ok(AutocompleteClan {
        tag: ClanTag::from(clan_tag),
        region,
    })
}
