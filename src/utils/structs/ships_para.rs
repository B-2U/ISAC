use std::collections::HashMap;

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use unidecode::unidecode;

use crate::utils::{
    structs::{Ship, ShipId},
    IsacError, IsacInfo, LoadSaveFromJson,
};

/// the struct for laoding ships_para.json
#[derive(Debug, Deserialize, Serialize)]
pub struct ShipsPara(pub HashMap<ShipId, Ship>);

impl LoadSaveFromJson for ShipsPara {
    const PATH: &'static str = "./web_src/ship/ships_para.json";
}

impl ShipsPara {
    /// shortcut to self.0.get, looking for the ship with ship_id
    pub fn get(&self, ship_id: &ShipId) -> Option<&Ship> {
        self.0.get(ship_id)
    }

    /// the combination of `normal_search()` and `fuzzy_search()`,
    ///
    /// use `normal_search()` at first, and do fuzzy search if no ship matched
    pub fn search_name(&self, input: &str, len_limit: usize) -> Result<Vec<Ship>, IsacError> {
        if input.is_empty() {
            return Ok(vec![]);
        }
        if let Some(candidates) = self.normal_search_name(input, len_limit) {
            return Ok(candidates);
        };
        if let Some(candidates) = self.fuzzy_search_name(input, len_limit) {
            return Ok(candidates);
        };
        Err(IsacInfo::ShipNotFound {
            ship_name: input.to_string(),
        })?
    }

    /// literal matching
    pub fn normal_search_name(&self, input: &str, len_limit: usize) -> Option<Vec<Ship>> {
        let input = input.to_lowercase();
        let candidates: Vec<_> = self
            .0
            .values()
            .filter(|ship| ship.is_available())
            .filter_map(|ship| {
                unidecode(&ship.name.to_lowercase())
                    .find(&input)
                    .map(|prefix_len| (ship, prefix_len))
            })
            .sorted_by_key(|(ship, prefix_len)| (*prefix_len, ship.name.len()))
            .map(|(ship, _)| ship)
            .take(len_limit)
            .cloned()
            .collect();
        match candidates.is_empty() {
            true => None,
            false => Some(candidates),
        }
    }
    /// fuzzy searching with Skim algorithm
    pub fn fuzzy_search_name(&self, input: &str, len_limit: usize) -> Option<Vec<Ship>> {
        let matcher = SkimMatcherV2::default();
        let candidates: Vec<_> = self
            .0
            .values()
            .filter(|ship| ship.is_available())
            .filter_map(|ship| {
                matcher
                    .fuzzy_match(&unidecode(&ship.name), input)
                    .map(|score| (score, ship))
            })
            .sorted_by(|a, b| Ord::cmp(&b.0, &a.0))
            .map(|(_, ship)| ship.clone())
            .take(len_limit)
            .collect();
        match candidates.is_empty() {
            true => None,
            false => Some(candidates),
        }
    }
}

impl From<ShipsPara> for HashMap<ShipId, Ship> {
    fn from(value: ShipsPara) -> Self {
        value.0
    }
}
