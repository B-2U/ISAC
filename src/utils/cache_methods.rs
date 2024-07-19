use std::num::NonZeroUsize;

use lru::LruCache;
use once_cell::sync::Lazy;
use parking_lot::Mutex;

use crate::{
    dc_utils::auto_complete::AutoCompleteClan,
    structs::{PartialClan, PartialPlayer, Region},
    utils::{wws_api::WowsApi, IsacError, IsacInfo},
};

/// searching player with the ign, with LRU cache
///
/// # Error
/// [`IsacInfo::PlayerIgnNotFound`]
pub async fn player(
    api: &WowsApi<'_>,
    region: &Region,
    ign: &str,
) -> Result<PartialPlayer, IsacError> {
    static CACHE: Lazy<Mutex<LruCache<(Region, String), PartialPlayer>>> =
        Lazy::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(30).unwrap())));

    let cache_result = {
        let mut lock = CACHE.lock();
        lock.get(&(*region, ign.to_string())).cloned()
    };
    if let Some(cached_player) = cache_result {
        Ok(cached_player)
    } else {
        let candidates = api.players(region, ign, 1).await?;
        let first_candidate = candidates.first().ok_or(IsacInfo::PlayerIgnNotFound {
            ign: ign.to_string(),
            region: *region,
        })?;
        let partial_player = PartialPlayer {
            region: *region,
            uid: first_candidate.uid,
        };
        CACHE
            .lock()
            .put((*region, first_candidate.name.clone()), partial_player);

        Ok(partial_player)
    }
}

/// searching clan with the tag, with LRU cache
///
/// # Error
/// [`IsacInfo::ClanNotFound`]
pub async fn clan(
    api: &WowsApi<'_>,
    auto_complete_clan: AutoCompleteClan,
) -> Result<PartialClan, IsacError> {
    static CACHE: Lazy<Mutex<LruCache<AutoCompleteClan, PartialClan>>> =
        Lazy::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(30).unwrap())));

    let cache_result = {
        let mut lock = CACHE.lock();
        lock.get(&auto_complete_clan).cloned()
    };
    if let Some(cached_clan) = cache_result {
        Ok(cached_clan)
    } else {
        let mut candidates = api
            .clans(&auto_complete_clan.region, &auto_complete_clan.tag)
            .await?;
        let first_candidate = match candidates.is_empty() {
            true => Err(IsacInfo::ClanNotFound {
                clan: auto_complete_clan.tag.to_string(),
                region: auto_complete_clan.region,
            })?,
            false => candidates.swap_remove(0),
        };
        CACHE
            .lock()
            .put(auto_complete_clan, first_candidate.clone());

        Ok(first_candidate)
    }
}
