use std::sync::Arc;

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::utils::structs::{ExpectedJs, ShipId};

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

impl Statistic {
    pub fn new<T: Into<StatisticValue>>(
        battles: u64,
        winrate: T,
        dmg: T,
        frags: T,
        planes: T,
        pr: T,
        exp: T,
        potential: u64,
        scout: u64,
        hitrate: f64,
    ) -> Self {
        Self {
            battles,
            winrate: winrate.into(),
            dmg: dmg.into(),
            frags: frags.into(),
            planes: planes.into(),
            pr: pr.into(),
            exp: exp.into(),
            potential,
            scout,
            hitrate,
        }
    }
}

// QA value actually can be u64 or f64, better way than String?
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatisticValue {
    pub value: String,
    pub color: String, // it's actually a hex color code in string
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
    fn _round_int(x: f64) -> u64 {
        x.round() as u64
    }
}

impl Default for StatisticValue {
    fn default() -> Self {
        Self {
            value: "0".to_string(),
            color: "#999999".to_string(),
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
        expected_js: &'a Arc<RwLock<ExpectedJs>>,
        value: f64,
        ship_id: &'a ShipId,
    },
}

impl From<StatisticValueType<'_>> for StatisticValue {
    fn from(value: StatisticValueType<'_>) -> StatisticValue {
        let (value_str, color) = match value {
            StatisticValueType::Winrate { value } => {
                const MAP: Lazy<Vec<(f64, &str)>> = Lazy::new(|| {
                    vec![
                        (65.0, "#9d42f3"),
                        (60.0, "#d042f3"),
                        (56.0, "#02c9b3"),
                        (54.0, "#318000"),
                        (52.0, "#44b300"),
                        (49.0, "#ffc71f"),
                        (47.0, "#fe7903"),
                        (-1.0, "#fe0e00"),
                    ]
                });
                let color = MAP
                    .iter()
                    .find(|(v, _)| &value >= v)
                    .map(|(_, color)| *color)
                    .unwrap_or("#fff");
                (Self::_round_2(value).to_string(), color.to_string())
            }
            StatisticValueType::Frags { value } => {
                const MAP: Lazy<Vec<(f64, &str)>> = Lazy::new(|| {
                    vec![
                        (1.44, "#d042f3"),
                        (1.2, "#02c9b3"),
                        (0.9, "#44b300"),
                        (0.73, "#ffc71f"),
                        (0.51, "#fe7903"),
                        (-1.0, "#fe0e00"),
                    ]
                });
                let color = MAP
                    .iter()
                    .find(|(v, _)| &value >= v)
                    .map(|(_, color)| *color)
                    .unwrap_or("#fff");
                (Self::_round_2(value).to_string(), color.to_string())
            }
            StatisticValueType::Planes { value } => {
                const MAP: Lazy<Vec<(f64, &str)>> = Lazy::new(|| {
                    vec![
                        (6.06, "#d042f3"),
                        (3.7, "#02c9b3"),
                        (1.8, "#44b300"),
                        (0.97, "#ffc71f"),
                        (0.22, "#fe7903"),
                        (-1.0, "#fe0e00"),
                    ]
                });
                let color = MAP
                    .iter()
                    .find(|(v, _)| &value >= v)
                    .map(|(_, color)| *color)
                    .unwrap_or("#fff");
                (Self::_round_2(value).to_string(), color.to_string())
            }
            StatisticValueType::Pr { value } => {
                const MAP: Lazy<Vec<(i64, &str)>> = Lazy::new(|| {
                    vec![
                        (2450, "#9d42f3"),
                        (2100, "#d042f3"),
                        (1750, "#02c9b3"),
                        (1550, "#318000"),
                        (1350, "#44b300"),
                        (1100, "#ffc71f"),
                        (750, "#fe7903"),
                        (-1, "#fe0e00"),
                    ]
                });
                if let Some(value) = value {
                    let color = MAP
                        .iter()
                        .find(|(v, _)| &(value as i64) >= v)
                        .map(|(_, color)| *color)
                        .unwrap_or("#fff");
                    (Self::_round_int(value).to_string(), color.to_string())
                } else {
                    ("N/A".to_string(), "#999999".to_string())
                }
            }
            StatisticValueType::OverallDmg { value } => {
                const MAP: Lazy<Vec<(i64, &str)>> = Lazy::new(|| {
                    vec![
                        (48500, "#d042f3"),
                        (38000, "#02c9b3"),
                        (28500, "#44b300"),
                        (23000, "#ffc71f"),
                        (16000, "#fe7903"),
                        (-1, "#fe0e00"),
                    ]
                });
                let color = MAP
                    .iter()
                    .find(|(v, _)| &(value as i64) >= v)
                    .map(|(_, color)| *color)
                    .unwrap_or("#fff");
                (Self::_round_int(value).to_string(), color.to_string())
            }
            StatisticValueType::ShipDmg {
                expected_js,
                value,
                ship_id,
            } => {
                const MAP: Lazy<Vec<(f64, &str)>> = Lazy::new(|| {
                    vec![
                        (1.7, "#d042f3"),
                        (1.3, "#02c9b3"),
                        (0.9, "#44b300"),
                        (0.65, "#ffc71f"),
                        (0.35, "#fe7903"),
                        (-1.0, "#fe0e00"),
                    ]
                });
                let color = if let Some(expected) = expected_js.read().data.get(&ship_id.0) {
                    let normal_value = f64::max(0.0, value as f64 / expected.dmg - 0.4) / 0.6;
                    MAP.iter()
                        .find(|(v, _)| &normal_value >= v)
                        .map(|(_, color)| *color)
                        .unwrap_or("#fff")
                } else {
                    "999999" // ship doesn't have expected value yet
                };

                (Self::_round_int(value).to_string(), color.to_string())
            }
            StatisticValueType::Exp { value } => {
                const MAP: Lazy<Vec<(i64, &str)>> = Lazy::new(|| {
                    vec![
                        (1500, "#9d42f3"),
                        (1350, "#d042f3"),
                        (1200, "#02c9b3"),
                        (1050, "#318000"),
                        (900, "#44b300"),
                        (750, "#ffc71f"),
                        (600, "#fe7903"),
                        (-1, "#fe0e00"),
                    ]
                });
                let color = MAP
                    .iter()
                    .find(|(v, _)| &(value as i64) >= v)
                    .map(|(_, color)| *color)
                    .unwrap_or("#fff");
                (Self::_round_int(value).to_string(), color.to_string())
            }
        };
        StatisticValue {
            value: value_str,
            color,
        }
    }
}
