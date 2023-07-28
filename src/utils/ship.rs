use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Ship {
    pub tier: u32,
    pub class: ShipClass,
    pub name: String,
    pub short_name: String,
    pub nation: String,
    pub icon: String,
}
// why is_available() never used?
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

#[derive(Debug, Deserialize, Serialize)]
pub struct ShipsPara(pub HashMap<u32, Ship>);

impl From<ShipsPara> for HashMap<u32, Ship> {
    fn from(value: ShipsPara) -> Self {
        value.0
    }
}
