use std::{collections::HashMap, fmt::Display, sync::Arc};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_repr::Deserialize_repr;
use strum::{EnumIter, IntoEnumIterator};

use crate::{
    utils::{
        structs::{api, ExpectedJs, Mode, ShipExpected, ShipsPara, Statistic, StatisticValueType},
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

#[allow(clippy::upper_case_acronyms)]
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

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ShipStatsCollection(pub HashMap<ShipId, ShipModeStatsPair>);

impl TryFrom<VortexShipAPIRes> for ShipStatsCollection {
    type Error = IsacError;

    fn try_from(mut value: VortexShipAPIRes) -> Result<Self, Self::Error> {
        if !value.status.ok() {
            return Err(IsacInfo::APIError {
                msg: value.status.err_msg(),
            }
            .into());
        }
        let player_stats = value.data.values_mut().last().ok_or(IsacInfo::APIError {
            msg: "expected PlayerStats".to_owned(),
        })?;
        if player_stats.hidden_profile.is_some() || player_stats.statistics.is_none() {
            Err(IsacInfo::PlayerHidden {
                ign: player_stats.name.clone(),
            }
            .into())
        } else {
            Ok(player_stats.statistics.take().unwrap()) // the if `statistics.is_none()` above already handle the None possibility
        }
    }
}

impl ShipStatsCollection {
    /// merging the responses from vortex
    pub fn merge(mut self, mut other: Self) -> Self {
        for (ship_id, main_pair) in self.0.iter_mut() {
            if let Some(sub_pair) = other.0.remove(ship_id) {
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
        use StatisticValueType as S;
        let winrate = S::Winrate {
            value: ttl_wins as f64 / battles as f64 * 100.0,
        }
        .into();
        let dmg = S::OverallDmg {
            value: ttl_dmg as f64 / battles as f64,
        }
        .into();
        let frags = S::Frags {
            value: ttl_frags as f64 / battles as f64,
        }
        .into();
        let planes = S::Planes {
            value: ttl_planes as f64 / battles as f64,
        }
        .into();
        let exp = S::Exp {
            value: ttl_exp as f64 / battles as f64,
        }
        .into();
        let pr = S::Pr {
            value: {
                let n_wr = f64::max(0.0, ttl_wins as f64 / exp_ttl_wins - 0.7) / 0.3;
                let n_dmg = f64::max(0.0, ttl_dmg as f64 / exp_ttl_dmg - 0.4) / 0.6;
                let n_frags = f64::max(0.0, ttl_frags as f64 / exp_ttl_frags - 0.1) / 0.9;
                Some(150.0 * n_wr + 700.0 * n_dmg + 300.0 * n_frags)
            },
        }
        .into();
        fn rounded_div(a: u64, b: u64) -> u64 {
            (a as f64 / b as f64).round() as u64
        }
        let potential = rounded_div(ttl_potential, battles);
        let scout = rounded_div(ttl_scout, battles);
        let hitrate = (hits as f64 / shots as f64 * 10000.0).round() / 100.0; // two decimal places

        Some(Statistic {
            battles,
            winrate,
            dmg,
            frags,
            planes,
            pr,
            exp,
            potential,
            scout,
            hitrate,
        })
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
#[derive(Default)]
pub struct ShipModeStatsPair(pub HashMap<Mode, ShipStats>);

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
        let stats = self.get(&mode)?;
        let battles = stats.battles_count;

        let winrate = stats.wins as f64 / battles as f64 * 100.0;
        let dmg = stats.damage_dealt as f64 / battles as f64;
        let frags = stats.frags as f64 / battles as f64;
        let planes = stats.planes_killed as f64 / battles as f64;
        let exp = stats.original_exp as f64 / battles as f64;
        let potential = stats.art_agro as f64 / battles as f64;
        let scout = stats.scouting_damage as f64 / battles as f64;
        let hitrate = if stats.shots_by_main != 0 {
            (stats.hits_by_main as f64 / stats.shots_by_main as f64 * 10000.0).round() / 100.0
        } else {
            0.0
        };
        let pr = expected_js.read().data.get(&ship_id.0).map(|expected| {
            let n_wr = f64::max(0.0, winrate / expected.winrate - 0.7) / 0.3;
            let n_dmg = f64::max(0.0, dmg / expected.dmg - 0.4) / 0.6;
            let n_frags = f64::max(0.0, frags / expected.frags - 0.1) / 0.9;
            150.0 * n_wr + 700.0 * n_dmg + 300.0 * n_frags
        });
        use StatisticValueType as S;
        Some(Statistic {
            battles,
            winrate: S::Winrate { value: winrate }.into(),
            dmg: S::ShipDmg {
                expected_js,
                value: dmg,
                ship_id,
            }
            .into(),
            frags: S::Frags { value: frags }.into(),
            planes: S::Planes { value: planes }.into(),
            pr: S::Pr { value: pr }.into(),
            exp: S::Exp { value: exp }.into(),
            potential: potential.round() as u64,
            scout: scout.round() as u64,
            hitrate,
        })
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

#[derive(Debug, Deserialize)]
pub struct PlayerStats {
    pub statistics: Option<ShipStatsCollection>,
    pub name: String, // player IGN. Don't care.
    pub hidden_profile: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct VortexShipAPIRes {
    #[serde(flatten)]
    pub status: api::Status,
    pub data: HashMap<String, PlayerStats>, // key is player UID. Don't care.
}

#[cfg(test)]
mod test {
    use super::{ShipStatsCollection, VortexShipAPIRes};

    #[tokio::test]
    async fn ship_stats_collection_hidden_can_deserialize() {
        let client = reqwest::Client::new();
        let res = client
            .get("https://vortex.worldofwarships.asia/api/accounts/2008493987/ships/")
            .send()
            .await
            .unwrap()
            .json::<ShipStatsCollection>()
            .await;
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
            .json::<VortexShipAPIRes>()
            .await
            .unwrap();
    }
}
