use crate::{
    utils::{
        structs::{PartialPlayer, Region},
        wws_api::WowsApi,
    },
    Context,
};

pub async fn ship(
    ctx: Context<'_>,
    input: &str,
) -> impl Iterator<Item = poise::AutocompleteChoice<u64>> {
    ctx.data()
        .ship_js
        .read()
        .search_name(input, 8)
        .unwrap_or_default()
        .into_iter()
        .map(|ship| poise::AutocompleteChoice {
            name: ship.name,
            value: ship.ship_id.0,
        })
}

/// return a serialized PartialPlayer struct
pub async fn player(
    ctx: Context<'_>,
    input: &str,
) -> impl Iterator<Item = poise::AutocompleteChoice<String>> {
    if input.is_empty() {
        vec![
            (
                "example: B2U",
                PartialPlayer {
                    region: Region::Asia,
                    uid: 2025455227,
                },
            ),
            (
                "example: asia B2U",
                PartialPlayer {
                    region: Region::Asia,
                    uid: 2025455227,
                },
            ),
            (
                "example: eu CVptsd",
                PartialPlayer {
                    region: Region::Eu,
                    uid: 566491687,
                },
            ),
        ]
        .into_iter()
        .map(|(name, value)| poise::AutocompleteChoice {
            name: name.to_string(),
            value: serde_json::to_string(&value).unwrap(),
        })
        .collect::<Vec<_>>()
        .into_iter()
    } else {
        let input: Vec<&str> = input.split_whitespace().collect();
        let (region, ign) = match input.len() {
            0 => unreachable!(
                "this should never happen, since we already deal with input.is_empty() aboved"
            ),
            1 => (Region::Asia, input[0]),
            _ => (Region::parse(input[0]).unwrap_or_default(), input[1]),
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
                poise::AutocompleteChoice {
                    name: format!("{} [{}]", vortex_p.name, region),
                    value: serde_json::to_string(&partial_p).unwrap(),
                }
            })
            .collect::<Vec<_>>()
            .into_iter()
    }
}
