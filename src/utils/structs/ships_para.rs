use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use unidecode::unidecode;

use crate::utils::{
    structs::{api, Ship, ShipClass, ShipId, ShipTier},
    IsacError, IsacInfo, LoadSaveFromJson,
};

/// the struct for laoding ships_para.json
#[derive(Debug, Deserialize, Serialize, Default)]
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
    ///
    /// It's guranteed to have at least one ship in the Vec<Ship>
    pub fn search_name(
        &self,
        input: &str,
        len_limit: usize,
    ) -> Result<ShipSearchCandidates, IsacError> {
        if input.is_empty() {
            return Err(IsacError::UnknownError(
                "search_name input is empty str".into(),
            ));
        }
        if let Some(candidates) = self.normal_search_name(input, len_limit) {
            return Ok(ShipSearchCandidates::new(candidates));
        };
        if let Some(candidates) = self.fuzzy_search_name(input, len_limit) {
            return Ok(ShipSearchCandidates::new(candidates));
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
impl From<HashMap<ShipId, Ship>> for ShipsPara {
    fn from(value: HashMap<ShipId, Ship>) -> Self {
        Self(value)
    }
}

/// Intermediate layer for `ShipsPara`
#[derive(Debug, Deserialize, Serialize)]
pub struct VortexVehicleAPIRes {
    #[serde(flatten)]
    status: api::Status,
    data: HashMap<ShipId, VehicleInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
struct VehicleInfo {
    level: ShipTier,
    /// e.g. PWSD990_Smalland
    name: String,
    icons: VehicleIcons,
    tags: Vec<String>,
    nation: String,
    localization: VehicleLocalization,
}

impl TryFrom<VortexVehicleAPIRes> for ShipsPara {
    type Error = IsacError;

    fn try_from(res: VortexVehicleAPIRes) -> Result<Self, Self::Error> {
        if !res.status.ok() {
            Err(IsacInfo::APIError {
                msg: res.status.err_msg(),
            })?
        } else {
            let output = res
                .data
                .into_iter()
                .map(|(k, mut v)| {
                    let class = v
                        .tags
                        .iter()
                        .find_map(|tag| ShipClass::from_tag(tag))
                        .expect("missing ship class tag");
                    let ship = Ship {
                        ship_id: k,
                        tier: v.level,
                        tier_roman: v.level.into(),
                        class,
                        name: v.localization.mark.remove("en").expect("missing en"),
                        short_name: v.localization.shortmark.remove("en").expect("missing en"),
                        nation: v.nation,
                        icon: v.icons.small,
                    };
                    (k, ship)
                })
                .collect::<HashMap<ShipId, Ship>>()
                .into();
            Ok(output)
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct VehicleIcons {
    local_contour: String,       //"gui/ship_bars/PWSD990_h_bg.png",
    contour_alive: String, //"vehicle/contour_alive/PWSD990_fb13534c0a2cea819b86d12a5da3cc6d955c115a177d0a28b640f653f9a4c3f7.png",
    medium: String, // "vehicle/medium/PWSD990_fb063a0b67bd1b296291c4a2d40311e65b93a4d69eeb62ad087668e32855d143.png",
    default: String, // "vehicle/small/PWSD990_6bb01eb076aca5179a1e3c8cbedd32490e77834cea155f7ab5db407378b23155.png",
    local_small: String, // "gui/ship_previews/PWSD990.png",
    contour_dead: String, // "vehicle/contour_dead/PWSD990_88949cd0601738e5c003da52c3b6fe85a37078723c15da3422d173e717f85c7d.png",
    large: String, // "vehicle/large/PWSD990_292ebab2e5a4e947697be86077329f45f66ee12952ccf05b6efb643c9e6d85d4.png",
    local_contour_dead: String, // "gui/ship_dead_icons/PWSD990.png",
    local_contour_alive: String, // "gui/ship_icons/PWSD990.png",
    small: String, // "vehicle/small/PWSD990_6bb01eb076aca5179a1e3c8cbedd32490e77834cea155f7ab5db407378b23155.png",
    contour: String, // "vehicle/contour/PWSD990_dc42e96bb81a5b83cdbe09b8862dab242085cdb2592195b6bccc7dabd5bdf329.png"
}

#[derive(Debug, Deserialize, Serialize)]
struct VehicleLocalization {
    mark: HashMap<String, String>,
    shortmark: HashMap<String, String>,
    description: HashMap<String, String>,
}

/// A shell of `Vec<Ship>` that gurantee 1 `Ship` in it
#[derive(Default)]
pub struct ShipSearchCandidates {
    inner: Vec<Ship>,
}

impl ShipSearchCandidates {
    pub fn new(candidates: Vec<Ship>) -> Self {
        Self { inner: candidates }
    }
    /// consume itself and return the first `Ship` inside, it's guranteed
    pub fn first(self) -> Ship {
        self.inner.into_iter().next().unwrap()
    }
}

impl Deref for ShipSearchCandidates {
    type Target = Vec<Ship>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for ShipSearchCandidates {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl IntoIterator for ShipSearchCandidates {
    type Item = Ship;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}
