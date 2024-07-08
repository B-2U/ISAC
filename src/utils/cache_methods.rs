use std::num::NonZeroUsize;

use lru::LruCache;
use once_cell::sync::Lazy;
use parking_lot::Mutex;

use crate::{
    structs::{PartialPlayer, Region},
    utils::{wws_api::WowsApi, IsacError, IsacInfo},
};

/// searching player with the ign, with LRU cache
///
/// # Error
/// [`IsacInfo::PlayerIgnNotFound`]
pub async fn player(
    api: WowsApi<'_>,
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