use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::structs::{color::ColorStats, ExpectedJs, ShipId};

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Statistic {
    pub battles: u64,
    pub winrate: StatisticValue,
    pub dmg: StatisticValue,
    pub frags: StatisticValue,
    pub planes: StatisticValue,
    pub pr: StatisticValue,
    pub exp: StatisticValue,
    pub potential: u64,
    pub scout: u64,
    pub hitrate: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatisticValue {
    pub value: f64,
    pub color: ColorStats,
}

impl StatisticValue {
    // fn _round(x: f64, decimals: u64) -> f64 {
    //     let y = 10_i32.pow(decimals) as f64;
    //     (x * y).round() / y
    // }
    /// round to two decimal places
    fn _round_2(x: f64) -> f64 {
        (x * 100.0).round() / 100.0
    }

    /// round up to int
    fn _round(x: f64) -> f64 {
        x.round()
    }
}

impl Default for StatisticValue {
    fn default() -> Self {
        Self {
            value: 0.0,
            color: ColorStats::Grey,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum StatisticValueType<'a> {
    /// remember * 100.0 first, e.g. 75.5% use 75.5
    Winrate {
        value: f64,
    },
    Frags {
        value: f64,
    },
    Planes {
        value: f64,
    },
    Pr {
        value: Option<f64>,
    },
    Exp {
        value: f64,
    },
    OverallDmg {
        value: f64,
    },
    ShipDmg {
        expected_js: &'a RwLock<ExpectedJs>,
        value: f64,
        ship_id: &'a ShipId,
    },
}
impl From<StatisticValueType<'_>> for StatisticValue {
    fn from(value: StatisticValueType<'_>) -> StatisticValue {
        let (value, color) = match value {
            StatisticValueType::Winrate { value } => {
                static MAP: Lazy<Vec<(f64, ColorStats)>> = Lazy::new(|| {
                    vec![
                        (65.0, ColorStats::SuperUnicum),
                        (60.0, ColorStats::Unicum),
                        (56.0, ColorStats::Great),
                        (54.0, ColorStats::VeryGood),
                        (52.0, ColorStats::Good),
                        (49.0, ColorStats::Average),
                        (47.0, ColorStats::BelowAverage),
                        (-1.0, ColorStats::Bad),
                    ]
                });
                let color = MAP
                    .iter()
                    .find(|(v, _)| &value >= v)
                    .map(|(_, color)| *color)
                    .unwrap_or(ColorStats::White);
                (Self::_round_2(value), color)
            }
            StatisticValueType::Frags { value } => {
                static MAP: Lazy<Vec<(f64, ColorStats)>> = Lazy::new(|| {
                    vec![
                        (1.44, ColorStats::Unicum),
                        (1.2, ColorStats::Great),
                        (0.9, ColorStats::Good),
                        (0.73, ColorStats::Average),
                        (0.51, ColorStats::BelowAverage),
                        (-1.0, ColorStats::Bad),
                    ]
                });
                let color = MAP
                    .iter()
                    .find(|(v, _)| &value >= v)
                    .map(|(_, color)| *color)
                    .unwrap_or(ColorStats::White);
                (Self::_round_2(value), color)
            }
            StatisticValueType::Planes { value } => {
                static MAP: Lazy<Vec<(f64, ColorStats)>> = Lazy::new(|| {
                    vec![
                        (6.06, ColorStats::Unicum),
                        (3.7, ColorStats::Great),
                        (1.8, ColorStats::Good),
                        (0.97, ColorStats::Average),
                        (0.22, ColorStats::BelowAverage),
                        (-1.0, ColorStats::Bad),
                    ]
                });
                let color = MAP
                    .iter()
                    .find(|(v, _)| &value >= v)
                    .map(|(_, color)| *color)
                    .unwrap_or(ColorStats::White);
                (Self::_round_2(value), color)
            }
            StatisticValueType::Pr { value } => {
                static MAP: Lazy<Vec<(i64, ColorStats)>> = Lazy::new(|| {
                    vec![
                        (2450, ColorStats::SuperUnicum),
                        (2100, ColorStats::Unicum),
                        (1750, ColorStats::Great),
                        (1550, ColorStats::VeryGood),
                        (1350, ColorStats::Good),
                        (1100, ColorStats::Average),
                        (750, ColorStats::BelowAverage),
                        (-1, ColorStats::Bad),
                    ]
                });
                if let Some(value) = value {
                    let color = MAP
                        .iter()
                        .find(|(v, _)| &(value as i64) >= v)
                        .map(|(_, color)| *color)
                        .unwrap_or(ColorStats::White);
                    (value.round(), color)
                } else {
                    (0.0, ColorStats::Grey)
                }
            }
            StatisticValueType::OverallDmg { value } => {
                static MAP: Lazy<Vec<(i64, ColorStats)>> = Lazy::new(|| {
                    vec![
                        (48500, ColorStats::Unicum),
                        (38000, ColorStats::Great),
                        (28500, ColorStats::Good),
                        (23000, ColorStats::Average),
                        (16000, ColorStats::BelowAverage),
                        (-1, ColorStats::Bad),
                    ]
                });
                let color = MAP
                    .iter()
                    .find(|(v, _)| &(value as i64) >= v)
                    .map(|(_, color)| *color)
                    .unwrap_or(ColorStats::White);
                (value.round(), color)
            }
            StatisticValueType::ShipDmg {
                expected_js,
                value,
                ship_id,
            } => {
                static MAP: Lazy<Vec<(f64, ColorStats)>> = Lazy::new(|| {
                    vec![
                        (1.7, ColorStats::Unicum),
                        (1.3, ColorStats::Great),
                        (0.9, ColorStats::Good),
                        (0.65, ColorStats::Average),
                        (0.35, ColorStats::BelowAverage),
                        (-1.0, ColorStats::Bad),
                    ]
                });
                let color = if let Some(expected) = expected_js.read().data.get(&ship_id.0) {
                    let normal_value = f64::max(0.0, value / expected.dmg - 0.4) / 0.6;
                    MAP.iter()
                        .find(|(v, _)| &normal_value >= v)
                        .map(|(_, color)| *color)
                        .unwrap_or(ColorStats::White)
                } else {
                    ColorStats::Grey // ship doesn't have expected value yet
                };

                (value.round(), color)
            }
            StatisticValueType::Exp { value } => {
                static MAP: Lazy<Vec<(i64, ColorStats)>> = Lazy::new(|| {
                    vec![
                        (1500, ColorStats::SuperUnicum),
                        (1350, ColorStats::Unicum),
                        (1200, ColorStats::Great),
                        (1050, ColorStats::VeryGood),
                        (900, ColorStats::Good),
                        (750, ColorStats::Average),
                        (600, ColorStats::BelowAverage),
                        (-1, ColorStats::Bad),
                    ]
                });
                let color = MAP
                    .iter()
                    .find(|(v, _)| &(value as i64) >= v)
                    .map(|(_, color)| *color)
                    .unwrap_or(ColorStats::White);
                (value.round(), color)
            }
        };
        StatisticValue { value, color }
    }
}
