use std::{collections::HashMap, fmt::Display, sync::Arc};

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use itertools::Itertools;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_repr::Deserialize_repr;
use strum::{EnumIter, IntoEnumIterator};
use unidecode::unidecode;

use crate::{
    cmds::tools::SHIPS_PARA_PATH,
    utils::{
        structs::{ExpectedJs, Mode, Statistic, StatisticValueType},
        IsacError, IsacInfo, LoadFromJson,
    },
    Context,
};

#[derive(Debug, Deserialize, Serialize, Clone, Copy, EnumIter, PartialEq, Eq, Hash)]
pub enum ShipClass {
    #[serde(rename = "ss")]
    SS,
    #[serde(rename = "dd")]
    DD,
    #[serde(rename = "ca")]
    CA,
    #[serde(rename = "bb")]
    BB,
    #[serde(rename = "cv")]
    CV,
}

#[derive(Debug, Clone, Copy, Deserialize_repr, Serialize, EnumIter, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ShipTier {
    I = 1,
    II = 2,
    III = 3,
    IV = 4,
    V = 5,
    VI = 6,
    VII = 7,
    VIII = 8,
    IX = 9,
    X = 10,
    XI = 11,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Ship {
    pub ship_id: ShipId,
    pub tier: ShipTier,
    pub class: ShipClass,
    pub name: String,
    pub short_name: String,
    pub nation: String,
    pub icon: String,
}
impl Ship {
    /// false for those CB or old ships
    ///
    /// e.g. `Langley (< 23.01.2019)`, `[Moskva]`
    pub fn is_available(&self) -> bool {
        !self.name.contains(['[', '('])
    }
}
impl Display for Ship {
    /// ship's short name
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.short_name)
    }
}

/// the struct for laoding ships_para.json
#[derive(Debug, Deserialize, Serialize)]
pub struct ShipsPara(pub HashMap<ShipId, Ship>);

impl ShipsPara {
    pub fn new() -> Self {
        Self::load_json_sync(SHIPS_PARA_PATH).unwrap()
    }
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
            .sorted_by_key(|(_, prefix_len)| *prefix_len)
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
#[derive(Debug, Deserialize, Clone)]
pub struct ShipStatsCollection(pub HashMap<ShipId, ShipModeStatsPair>);

impl Default for ShipStatsCollection {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl TryFrom<VortexShipResponse> for ShipStatsCollection {
    type Error = IsacError;

    fn try_from(mut value: VortexShipResponse) -> Result<Self, Self::Error> {
        if let Some(err) = value.error {
            return Err(IsacInfo::APIError { msg: err }.into());
        };
        let player_stats = value.data.values_mut().last().ok_or(IsacInfo::APIError {
            msg: "expected PlayerStats".to_owned(),
        })?;
        if player_stats.hidden_profile.is_some() || player_stats.statistics.is_none() {
            return Err(IsacInfo::PlayerHidden {
                ign: player_stats.name.clone(),
            }
            .into());
        } else {
            Ok(player_stats.statistics.take().unwrap()) // the if `statistics.is_none()` above already handle the None possibility
        }
    }
}
impl ShipStatsCollection {
    /// merging the responses from vortex
    pub fn merge(mut self, mut other: Self) -> Self {
        for (ship_id, main_pair) in self.0.iter_mut() {
            if let Some(sub_pair) = other.0.remove(&ship_id) {
                main_pair.0.extend(sub_pair.0)
            }
        }
        self
    }
    /// consume Self and sort the given ships by their class
    pub fn sort_class(self, ctx: &Context<'_>) -> HashMap<ShipClass, ShipStatsCollection> {
        let mut map: HashMap<ShipClass, ShipStatsCollection> = ShipClass::iter()
            .map(|class| (class, ShipStatsCollection::default()))
            .collect();

        let ship_js = ctx.data().ship_js.read();
        for (ship_id, ship_modes) in self.0 {
            if let Some(class_collection) = ship_js
                .get(&ship_id)
                .and_then(|ship_para| map.get_mut(&ship_para.class))
            {
                class_collection.0.insert(ship_id, ship_modes);
            }
        }
        map
    }
    /// consume Self and sort the given ships by their class
    pub fn sort_tier(self, ctx: &Context<'_>) -> HashMap<ShipTier, ShipStatsCollection> {
        let mut map: HashMap<ShipTier, ShipStatsCollection> = ShipTier::iter()
            .map(|class| (class, ShipStatsCollection::default()))
            .collect();
        {
            let ship_js = ctx.data().ship_js.read();
            for (ship_id, ship_modes) in self.0 {
                if let Some(ship_para) = ship_js.get(&ship_id) {
                    if let Some(class_collection) = map.get_mut(&ship_para.tier) {
                        class_collection.0.insert(ship_id, ship_modes);
                    }
                }
            }
        }
        map
    }

    /// calculate the average stats with given ships
    pub fn to_statistic(&self, expected_js: &Arc<RwLock<ExpectedJs>>, mode: Mode) -> Statistic {
        let (
            battles,
            wins,
            ttl_dmg,
            ttl_frags,
            ttl_planes,
            ttl_exp,
            ttl_potential,
            ttl_scout,
            shots,
            hits,
            last_ship_id,
        ) = self.0.iter().fold(
            (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0),
            |acc, (ship_id, ship_modes)| {
                if let Some(Some(ship)) = ship_modes.0.get(&mode) {
                    (
                        acc.0 + ship.battles_count,
                        acc.1 + ship.wins,
                        acc.2 + ship.damage_dealt,
                        acc.3 + ship.frags,
                        acc.4 + ship.planes_killed,
                        acc.5 + ship.original_exp,
                        acc.6 + ship.art_agro,
                        acc.7 + ship.scouting_damage,
                        acc.8 + ship.shots_by_main,
                        acc.9 + ship.hits_by_main,
                        ship_id.0,
                    )
                } else {
                    acc
                }
            },
        );
        if battles == 0 {
            return Statistic::default();
        };
        use StatisticValueType::*;
        let winrate = Winrate {
            value: wins as f64 / battles as f64 * 100.0,
        };
        let dmg = match self.0.len() == 1 {
            true => ShipDmg {
                expected_js,
                value: ttl_dmg as f64 / battles as f64,
                ship_id: last_ship_id,
            },
            false => OverallDmg {
                value: ttl_dmg as f64 / battles as f64,
            },
        };
        let frags = Frags {
            value: ttl_frags as f64 / battles as f64,
        };
        let planes = Planes {
            value: ttl_planes as f64 / battles as f64,
        };
        let exp: StatisticValueType<'_> = Exp {
            value: ttl_exp as f64 / battles as f64,
        };
        let pr = Pr {
            value: self.pr(expected_js, mode),
        };
        fn rounded_div(a: u64, b: u64) -> u64 {
            (a as f64 / b as f64).round() as u64
        }
        let potential = rounded_div(ttl_potential, battles);
        let scout = rounded_div(ttl_scout, battles);
        let hitrate = (hits as f64 / shots as f64 * 10000.0).round() / 100.0; // two decimal places

        Statistic::new(
            battles, winrate, dmg, frags, planes, pr, exp, potential, scout, hitrate,
        )
    }

    /// calculate the average pr with given ships
    fn pr(&self, expected_js: &Arc<RwLock<ExpectedJs>>, mode: Mode) -> f64 {
        let (battles, wins, dmg, frags, exp_wins, exp_dmg, exp_frags) = {
            let exp_js_guard = expected_js.read();
            self.0
                .iter()
                .fold((0, 0, 0, 0, 0.0, 0.0, 0.0), |acc, (ship_id, ship_modes)| {
                    if let (Some(Some(ship)), Some(exp_value)) =
                        (ship_modes.0.get(&mode), exp_js_guard.data.get(&ship_id.0))
                    {
                        (
                            acc.0 + ship.battles_count,
                            acc.1 + ship.wins,
                            acc.2 + ship.damage_dealt,
                            acc.3 + ship.frags,
                            acc.4 + exp_value.winrate / 100.0 * ship.battles_count as f64,
                            acc.5 + exp_value.dmg * ship.battles_count as f64,
                            acc.6 + exp_value.frags * ship.battles_count as f64,
                        )
                    } else {
                        acc
                    }
                })
        };
        if battles == 0 {
            0.0
        } else {
            let n_wr = f64::max(0.0, wins as f64 / exp_wins - 0.7) / 0.3;
            let n_dmg = f64::max(0.0, dmg as f64 / exp_dmg - 0.4) / 0.6;
            let n_frags = f64::max(0.0, frags as f64 / exp_frags - 0.1) / 0.9;
            150.0 * n_wr + 700.0 * n_dmg + 300.0 * n_frags
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ShipId(pub u64);
impl ShipId {
    /// get [`Ship`] from ShipId, None if not found
    pub fn get_ship(&self, ctx: &Context<'_>) -> Option<Ship> {
        ctx.data().ship_js.read().get(&ShipId(self.0)).cloned()
    }
}

impl Display for ShipId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(try_from = "JsonValue")]
pub struct ShipModeStatsPair(pub HashMap<Mode, Option<ShipStats>>);
impl TryFrom<JsonValue> for ShipModeStatsPair {
    type Error = String;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        let map = value
            .as_object()
            .ok_or("Expected an object (map), but found something else.")?;

        let mut pairs = HashMap::new();
        for (key, value) in map.into_iter() {
            let mode =
                serde_json::from_value(key.as_str().into()).map_err(|err| err.to_string())?;

            // if no stats found for ship, value is an empty object
            let maybe_stats = {
                let stats_map = value
                    .as_object()
                    .ok_or("Expected an object (map), but found something else.")?;

                if stats_map.is_empty() {
                    None
                } else {
                    Some(serde_json::from_value(value.clone()).map_err(|err| err.to_string())?)
                }
            };

            pairs.insert(mode, maybe_stats);
        }

        Ok(Self(pairs))
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ShipStats {
    battles_count: u64,
    wins: u64,
    damage_dealt: u64,
    frags: u64,
    planes_killed: u64,
    original_exp: u64,
    art_agro: u64,
    scouting_damage: u64,
    shots_by_main: u64,
    hits_by_main: u64,
}

#[derive(Debug, Deserialize)]
pub struct PlayerStats {
    pub statistics: Option<ShipStatsCollection>,
    pub name: String, // player IGN. Don't care.
    pub hidden_profile: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct VortexShipResponse {
    pub status: String,                     // known values: `ok`, `error`.
    pub error: Option<String>,              // error message.
    pub data: HashMap<String, PlayerStats>, // key is player UID. Don't care.
}

#[cfg(test)]
mod test {
    use std::fs;

    use super::{ShipStatsCollection, VortexShipResponse};

    #[test]
    fn ship_stats_collection_hidden_can_deserialize() {
        let json = fs::read_to_string("test_res/vortex-ship-response-hidden.json").unwrap();
        let res: Result<ShipStatsCollection, serde_json::Error> = serde_json::from_str(&json);
        match res {
            Ok(_) => panic!("hidden player shouldnt be Ok()"),
            Err(_err) => {
                println!("{}", _err);
            }
        }
    }

    #[tokio::test]
    async fn vortex_ship_response_can_deserialize() {
        let client = reqwest::Client::new();
        let _response = client
            .get(
                "https://vortex.worldofwarships.asia/api/accounts/2025455227/ships/3530504176/pvp/",
            )
            .send()
            .await
            .unwrap()
            .json::<VortexShipResponse>()
            .await
            .unwrap();
    }
}
