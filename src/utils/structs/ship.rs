use std::{collections::HashMap, fmt::Display, sync::Arc};

use parking_lot::{RwLock, RwLockReadGuard};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_repr::Deserialize_repr;
use strum::{EnumIter, IntoEnumIterator};

use crate::{
    utils::{
        structs::{ExpectedJs, Mode, ShipExpected, ShipsPara, Statistic, StatisticValueType},
        IsacError, IsacInfo,
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

fn add_glossary_url_prefix<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    Ok(format!("https://glossary-wows-global.gcdn.co/icons/{}", s))
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Ship {
    pub ship_id: ShipId,
    pub tier: ShipTier,
    pub class: ShipClass,
    pub name: String,
    pub short_name: String,
    pub nation: String,
    #[serde(deserialize_with = "add_glossary_url_prefix")]
    pub icon: String,
}

impl Default for Ship {
    fn default() -> Self {
        Self {
            ship_id: ShipId(0),
            tier: ShipTier::X,
            class: ShipClass::DD,
            name: "Unknown Ship".to_string(),
            short_name: "Unknown Ship".to_string(),
            nation: Default::default(),
            icon: Default::default(),
        }
    }
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

#[derive(Debug, Deserialize, Serialize, Clone)]
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

    /// get the difference between 2 collections, return None if its empty
    ///
    /// **Note**: `self` need to be later, `other` be older
    pub fn compare(&self, mut other: Self) -> Option<Self> {
        // QA faster way
        let mut output_collection = ShipStatsCollection::default();
        for (ship_id, main_pair) in self.0.iter() {
            let mut output_pair = ShipModeStatsPair::default();
            let mut old_pair = other.0.remove(ship_id).unwrap_or_default();

            if main_pair == &old_pair {
                continue;
            }
            for mode in Mode::iter() {
                let Some(current) = main_pair.get(&mode) else {
                    continue; // no self
                };
                let old = old_pair.0.remove(&mode).unwrap_or_default();

                if current.battles_count <= old.battles_count {
                    continue; // no different or account got rollback
                };
                let diff_s = ShipStats {
                    battles_count: current.battles_count - old.battles_count,
                    wins: current.wins - old.wins,
                    damage_dealt: current.damage_dealt - old.damage_dealt,
                    frags: current.frags - old.frags,
                    planes_killed: current.planes_killed - old.planes_killed,
                    original_exp: current.original_exp - old.original_exp,
                    art_agro: current.art_agro - old.art_agro,
                    scouting_damage: current.scouting_damage - old.scouting_damage,
                    shots_by_main: current.shots_by_main - old.shots_by_main,
                    hits_by_main: current.hits_by_main - old.hits_by_main,
                };
                output_pair.0.insert(mode, diff_s);
            }
            if output_pair.0.is_empty() {
                continue;
            }
            output_collection.0.insert(*ship_id, output_pair);
        }
        if output_collection.0.is_empty() {
            None
        } else {
            Some(output_collection)
        }
    }

    /// remove those ships doesn't has self in all 4 modes
    pub fn clean(&mut self) -> &Self {
        self.0.retain(|_ship_id, s| !s.0.is_empty());
        self
    }

    /// shortcut to `self.0.retain`
    pub fn retain<P>(mut self, predicate: P) -> Self
    where
        Self: Sized,
        P: FnMut(&ShipId, &mut ShipModeStatsPair) -> bool,
    {
        self.0.retain(predicate);
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
    pub fn to_statistic(
        &self,
        expected_js: &Arc<RwLock<ExpectedJs>>,
        mode: Mode,
    ) -> Option<Statistic> {
        let (
            battles,
            ttl_wins,
            ttl_dmg,
            ttl_frags,
            ttl_planes,
            ttl_exp,
            ttl_potential,
            ttl_scout,
            shots,
            hits,
            exp_ttl_wins,
            exp_ttl_dmg,
            exp_ttl_frags,
        ) = {
            let guard = expected_js.read();
            let empty_ship_expected = ShipExpected {
                dmg: 0.0,
                frags: 0.0,
                winrate: 0.0,
            };
            self.0
                .iter()
                .filter_map(|(ship_id, ship_modes)| ship_modes.get(&mode).map(|s| (ship_id, s)))
                .fold(
                    (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0.0, 0.0, 0.0),
                    |acc, (ship_id, ship)| {
                        let ship_expected =
                            guard.data.get(&ship_id.0).unwrap_or(&empty_ship_expected); // QA its a reference, so i cant unwrap_or_default()
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
                            acc.10 + ship_expected.winrate / 100.0 * ship.battles_count as f64,
                            acc.11 + ship_expected.dmg * ship.battles_count as f64,
                            acc.12 + ship_expected.frags * ship.battles_count as f64,
                        )
                    },
                )
        };
        if battles == 0 {
            return None;
        };
        use StatisticValueType::*;
        let winrate = Winrate {
            value: ttl_wins as f64 / battles as f64 * 100.0,
        };
        let dmg = OverallDmg {
            value: ttl_dmg as f64 / battles as f64,
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
            value: {
                let n_wr = f64::max(0.0, ttl_wins as f64 / exp_ttl_wins - 0.7) / 0.3;
                let n_dmg = f64::max(0.0, ttl_dmg as f64 / exp_ttl_dmg - 0.4) / 0.6;
                let n_frags = f64::max(0.0, ttl_frags as f64 / exp_ttl_frags - 0.1) / 0.9;
                Some(150.0 * n_wr + 700.0 * n_dmg + 300.0 * n_frags)
            },
        };
        fn rounded_div(a: u64, b: u64) -> u64 {
            (a as f64 / b as f64).round() as u64
        }
        let potential = rounded_div(ttl_potential, battles);
        let scout = rounded_div(ttl_scout, battles);
        let hitrate = (hits as f64 / shots as f64 * 10000.0).round() / 100.0; // two decimal places

        Some(Statistic::new(
            battles, winrate, dmg, frags, planes, pr, exp, potential, scout, hitrate,
        ))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct ShipId(pub u64);
impl ShipId {
    /// get [`Ship`] from ShipId, None if not found
    // QA is borrowing lock idiomatic?
    pub fn get_ship(&self, ship_js: &RwLock<ShipsPara>) -> Option<Ship> {
        ship_js.read().get(&ShipId(self.0)).cloned()
    }
}

impl Display for ShipId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(try_from = "JsonValue")]
pub struct ShipModeStatsPair(pub HashMap<Mode, ShipStats>);

impl Default for ShipModeStatsPair {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl ShipModeStatsPair {
    /// a shorcut of `self.0.get
    fn get(&self, mode: &Mode) -> Option<&ShipStats> {
        self.0.get(mode)
    }
    /// calculate the statistic of the ship, None if battles = 0
    pub fn to_statistic(
        &self,
        ship_id: &ShipId,
        expected_js: &Arc<RwLock<ExpectedJs>>,
        mode: Mode,
    ) -> Option<Statistic> {
        let Some(s) = self.get(&mode) else {
            return None;
        };
        let avg = s.calc(ship_id, &expected_js.read());
        use StatisticValueType::*;
        let pr = avg.expected.map(|expected| {
            let n_wr = f64::max(0.0, avg.winrate / expected.winrate - 0.7) / 0.3;
            let n_dmg = f64::max(0.0, avg.dmg / expected.dmg - 0.4) / 0.6;
            let n_frags = f64::max(0.0, avg.frags / expected.frags - 0.1) / 0.9;
            150.0 * n_wr + 700.0 * n_dmg + 300.0 * n_frags
        });
        Some(Statistic::new(
            avg.battles,
            Winrate { value: avg.winrate },
            ShipDmg {
                expected_js,
                value: avg.dmg,
                ship_id,
            },
            Frags { value: avg.frags },
            Planes { value: avg.planes },
            Pr { value: pr },
            Exp { value: avg.exp },
            avg.potential.round() as u64,
            avg.scout.round() as u64,
            avg.hitrate,
        ))
    }
}

impl TryFrom<JsonValue> for ShipModeStatsPair {
    type Error = String;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        let map = value
            .as_object()
            .ok_or("Expected an object (map), but found something else.")?;

        let mut pairs = HashMap::new();
        for (key, value) in map.into_iter() {
            let mode: Mode =
                serde_json::from_value(key.as_str().into()).map_err(|err| err.to_string())?;

            // if no stats found for ship, value is an empty object
            let maybe_stats: Option<ShipStats> = {
                let stats_map = value
                    .as_object()
                    .ok_or("Expected an object (map), but found something else.")?;

                if stats_map.is_empty() {
                    None
                } else {
                    Some(serde_json::from_value(value.clone()).map_err(|err| err.to_string())?)
                }
            };
            if let Some(stats) = maybe_stats {
                pairs.insert(mode, stats);
            }
        }

        Ok(Self(pairs))
    }
}

/// the battles_count should never be 0
#[derive(Debug, Deserialize, Serialize, Clone, Default, PartialEq, Eq)]
pub struct ShipStats {
    #[serde(alias = "battles")] // for read old recent data
    battles_count: u64,
    wins: u64,
    damage_dealt: u64,
    frags: u64,
    planes_killed: u64,
    original_exp: u64,
    #[serde(default)]
    art_agro: u64,
    scouting_damage: u64,
    shots_by_main: u64,
    hits_by_main: u64,
}

impl ShipStats {
    /// get the datas needed for constructing [`Statistic`]
    pub fn calc(
        &self,
        ship_id: &ShipId,
        expected_js: &RwLockReadGuard<ExpectedJs>,
    ) -> ShipStatsAvg {
        let battles = self.battles_count;

        let winrate = self.wins as f64 / battles as f64 * 100.0;
        let dmg = self.damage_dealt as f64 / battles as f64;
        let frags = self.frags as f64 / battles as f64;
        let planes = self.planes_killed as f64 / battles as f64;
        let exp = self.original_exp as f64 / battles as f64;
        let potential = self.art_agro as f64 / battles as f64;
        let scout = self.scouting_damage as f64 / battles as f64;
        let hitrate = if self.shots_by_main != 0 {
            (self.hits_by_main as f64 / self.shots_by_main as f64 * 10000.0).round() / 100.0
        } else {
            0.0
        };
        ShipStatsAvg {
            battles,
            winrate,
            dmg,
            frags,
            planes,
            exp,
            potential,
            scout,
            hitrate,
            expected: expected_js.data.get(&ship_id.0).copied(),
        }
        // two decimal places
    }
}

// QA its actually really useless, used by only once method, and that method used only once too
/// a struct for holding calculated [`ShipStats`]
#[derive(Debug)]
pub struct ShipStatsAvg {
    battles: u64,
    winrate: f64, // already * 100
    dmg: f64,
    frags: f64,
    planes: f64,
    exp: f64,
    potential: f64,
    scout: f64,
    hitrate: f64,
    expected: Option<ShipExpected>,
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
