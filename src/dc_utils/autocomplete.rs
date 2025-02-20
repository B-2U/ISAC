use poise::serenity_prelude::AutocompleteChoice;
use serde::{Deserialize, Serialize};

use crate::{
    Context,
    structs::{ClanTag, PartialClan, Region},
    utils::wws_api::WowsApi,
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

/// return a formatted string like: B2U[ASIA]
pub async fn player(ctx: Context<'_>, input: &str) -> Vec<AutocompleteChoice> {
    let input: Vec<&str> = input.split_whitespace().collect();
    let Some((region, ign)) = (match input.len() {
        0 => None,
        1 => Some((Region::Asia, input[0])),
        _ => Some((Region::parse(input[0]).unwrap_or_default(), input[1])),
    }) else {
        return match ctx.data().cache.lock().await.get(&ctx.author().id).await {
            Some(cache) => cache
                .autocomplete_player
                .iter()
                .map(|p| AutocompleteChoice::new(p.clone(), p.clone()))
                .collect(),
            None => [
                ["Usage: [region] <ign>", "Usage: [region] <ign>"],
                ["Example: B2U", "B2U (ASIA)"],
                ["Example: asia B2U", "B2U (ASIA)"],
                ["Example: eu CVptsd", "CVptsd (EU)"],
            ]
            .into_iter()
            .map(|[display, input]| AutocompleteChoice::new(display.to_string(), input.to_string()))
            .collect(),
        };
    };
    let candidates = WowsApi::new(&ctx)
        .players(&region, ign, 8)
        .await
        .unwrap_or_default();
    candidates
        .into_iter()
        .map(|vortex_p| {
            let autocomplete_p = vortex_p.into_autocomplete_player(region);
            AutocompleteChoice::new(autocomplete_p.clone(), autocomplete_p.clone())
        })
        .collect()
}

/// return a formatted string, e.g. [PANTS] Dont Cap Kill All (ASIA)
pub async fn clan(ctx: Context<'_>, input: &str) -> Vec<AutocompleteChoice> {
    let input: Vec<&str> = input.split_whitespace().collect();
    let Some((region, clan_name)) = (match input.len() {
        0 => None,
        1 => Some((Region::Asia, input[0])),
        _ => Some((Region::parse(input[0]).unwrap_or_default(), input[1])),
    }) else {
        return [
            ("Example: VOR", "[PANTS] (ASIA)"),
            ("Example: EU RAIN", "[RAIN] (EU)"),
            ("Example: NA RESIN", "[RESIN] (NA)"),
        ]
        .into_iter()
        .map(|(name, value)| AutocompleteChoice::new(name, value))
        .collect();
    };
    let api = WowsApi::new(&ctx);
    let candidates = api.clans(&region, clan_name).await.unwrap_or_default();
    candidates
        .into_iter()
        .map(|clan| {
            let formatted_str = format!("[{}] {} ({})", clan, clan.name, clan.region);
            AutocompleteChoice::new(formatted_str.clone(), formatted_str)
        })
        .collect()
}

/// a temp struct for passing autocomplete result back due to the value size limit (100), can be removed if there's a better way lik command data()
#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq)]
pub struct AutocompleteClan {
    pub tag: ClanTag,
    pub region: Region,
}

impl From<PartialClan> for AutocompleteClan {
    fn from(v: PartialClan) -> Self {
        Self {
            tag: v.tag,
            region: v.region,
        }
    }
}
