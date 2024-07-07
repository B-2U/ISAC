use poise::serenity_prelude::AutocompleteChoice;
use serde::{Deserialize, Serialize};

use crate::{
    structs::{ClanTag, PartialClan, PartialPlayer, Region},
    utils::wws_api::WowsApi,
    Context,
};

pub async fn ship(ctx: Context<'_>, input: &str) -> impl Iterator<Item = AutocompleteChoice> {
    ctx.data()
        .ships
        .read()
        .search_name(input, 8)
        .unwrap_or_default()
        .into_iter()
        .map(|ship| AutocompleteChoice::new(ship.name.as_str(), ship.name.as_str()))
}

/// return a serialized [`PartialPlayer`] struct
pub async fn player(ctx: Context<'_>, input: &str) -> Vec<AutocompleteChoice> {
    let input: Vec<&str> = input.split_whitespace().collect();
    let Some((region, ign)) = (match input.len() {
        0 => None,
        1 => Some((Region::Asia, input[0])),
        _ => Some((Region::parse(input[0]).unwrap_or_default(), input[1])),
    }) else {
        return [
            (
                "Usage: <region(optional)> <ign>",
                PartialPlayer {
                    region: Region::Asia,
                    uid: 2025455227,
                },
            ),
            (
                "Example: B2U",
                PartialPlayer {
                    region: Region::Asia,
                    uid: 2025455227,
                },
            ),
            (
                "Example: asia B2U",
                PartialPlayer {
                    region: Region::Asia,
                    uid: 2025455227,
                },
            ),
            (
                "Example: eu CVptsd",
                PartialPlayer {
                    region: Region::Eu,
                    uid: 566491687,
                },
            ),
            (
                "Example: NA JustDodge",
                PartialPlayer {
                    region: Region::Na,
                    uid: 1035252322,
                },
            ),
        ]
        .into_iter()
        .map(|(name, value)| {
            AutocompleteChoice::new(name.to_string(), serde_json::to_string(&value).unwrap())
        })
        .collect();
    };
    let api = WowsApi::new(&ctx);
    let candidates = api.players(&region, ign, 8).await.unwrap_or_default();
    candidates
        .into_iter()
        .map(|vortex_p| {
            let partial_p = PartialPlayer {
                region,
                uid: vortex_p.uid,
            };
            AutocompleteChoice::new(
                format!("{} [{}]", vortex_p.name, region),
                serde_json::to_string(&partial_p).unwrap(),
            )
        })
        .collect()
}

/// return a serialized [`AutoCompleteClan`] struct
pub async fn clan(ctx: Context<'_>, input: &str) -> Vec<AutocompleteChoice> {
    let input: Vec<&str> = input.split_whitespace().collect();
    let Some((region, clan_name)) = (match input.len() {
        0 => None,
        1 => Some((Region::Asia, input[0])),
        _ => Some((Region::parse(input[0]).unwrap_or_default(), input[1])),
    }) else {
        return [
            (
                "Example: VOR",
                AutoCompleteClan {
                    tag: ClanTag::from("VOR"),
                    region: Region::Asia,
                },
            ),
            (
                "Example: EU RAIN",
                AutoCompleteClan {
                    tag: ClanTag::from("RAIN"),
                    region: Region::Eu,
                },
            ),
            (
                "Example: NA RESIN",
                AutoCompleteClan {
                    tag: ClanTag::from("RESIN"),
                    region: Region::Na,
                },
            ),
        ]
        .into_iter()
        .map(|(name, value)| {
            AutocompleteChoice::new(name.to_string(), serde_json::to_string(&value).unwrap())
        })
        .collect();
    };
    let api = WowsApi::new(&ctx);
    let candidates = api.clans(&region, clan_name).await.unwrap_or_default();
    candidates
        .into_iter()
        .map(|clan| {
            let auto_complete_clan: AutoCompleteClan = clan.clone().into();
            AutocompleteChoice::new(
                format!("[{}] {} ({})", clan, clan.name, clan.region),
                serde_json::to_string(&auto_complete_clan).unwrap(),
            )
        })
        .collect()
}

/// a temp struct for passing autocomplete result back due to the value size limit (100), can be removed if there's a better way lik command data()
#[derive(Serialize, Deserialize, Debug)]
pub struct AutoCompleteClan {
    pub tag: ClanTag,
    pub region: Region,
}

impl From<PartialClan> for AutoCompleteClan {
    fn from(v: PartialClan) -> Self {
        Self {
            tag: v.tag,
            region: v.region,
        }
    }
}
