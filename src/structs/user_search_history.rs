use std::{num::NonZeroUsize, path::PathBuf};

use lru::LruCache;
use poise::serenity_prelude::UserId;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::structs::{lru_vector::LruVector, AutocompletePlayer};

/// users searching history in autocomplete::player()
/// user's data is saved when getting evicted or bot shutting down
#[derive(Debug)]
pub struct SearchCache {
    pub users: LruCache<UserId, UserSearchCache>,
}

impl SearchCache {
    pub fn new() -> Self {
        SearchCache {
            users: LruCache::new(NonZeroUsize::new(50).unwrap()),
        }
    }
    /// search in cache first then disk, None if it's not exist
    pub async fn get(&mut self, user_id: &UserId) -> Option<&UserSearchCache> {
        // no cache, try load from disk first
        if self.users.get(user_id).is_none() {
            if let Some(cache) = UserSearchCache::load(user_id).await {
                // if someone evicted, save it
                if let Some((evicted_id, evicted_data)) = self.users.push(*user_id, cache) {
                    if &evicted_id != user_id {
                        evicted_data.save().await;
                    }
                };
            } else {
                return None;
            };
        };
        self.users.get(user_id)
    }

    /// search in cache first then disk, insert if no cache
    // pub async fn get_or_insert<F>(&mut self, user_id: &UserId, f: F) -> &UserSearchCache
    // where
    //     F: FnOnce() -> UserSearchCache,
    // {
    //     // no cache, try load from disk first
    //     if self.users.get(user_id).is_none() {
    //         let cache = match UserSearchCache::load(user_id).await {
    //             Some(cache) => cache,
    //             None => f(),
    //         };
    //         // if someone evicted, save it
    //         if let Some((evicted_id, evicted_data)) = self.users.push(*user_id, cache) {
    //             if &evicted_id != user_id {
    //                 evicted_data.save().await;
    //             }
    //         };
    //     };
    //     self.users.get(user_id).expect("we put it in above already")
    // }

    /// search in cache then disk. If the key does not exist the provided FnOnce is used to populate the list and a mutable reference is returned.
    pub async fn get_or_insert_mut<F>(&mut self, user_id: &UserId, f: F) -> &mut UserSearchCache
    where
        F: FnOnce() -> UserSearchCache,
    {
        // no cache, try load from disk first
        if self.users.get(user_id).is_none() {
            let cache = match UserSearchCache::load(user_id).await {
                Some(cache) => cache,
                None => f(),
            };
            // if someone evicted, save it
            self.push_save(*user_id, cache).await;
        };
        self.users
            .get_mut(user_id)
            .expect("we put it in above already")
    }
    /// insert new child, save the evicted one to disk
    pub async fn push_save(&mut self, user_id: UserId, cache: UserSearchCache) {
        if let Some((evicted_id, evicted_data)) = self.users.push(user_id, cache) {
            if evicted_id != user_id {
                evicted_data.save().await;
            }
        };
    }
}

/// the `String` is players ign
#[derive(Debug, Serialize, Deserialize)]
pub struct UserSearchCache {
    pub user_id: UserId,
    pub autocomplete_player: LruVector<AutocompletePlayer>,
}

impl UserSearchCache {
    pub fn new(user_id: UserId) -> Self {
        let autocomplete_player = LruVector::new(15);
        UserSearchCache {
            user_id,
            autocomplete_player,
        }
    }
    /// load the player's recent data, return None if he is not inside
    pub async fn load(user_id: &UserId) -> Option<Self> {
        let path = Self::get_path(user_id);
        // std::fs::File::open() is as fast as path.exists()
        if let Ok(file) = std::fs::File::open(&path) {
            let mut data: Self = tokio::task::spawn_blocking(move || {
                let reader = std::io::BufReader::new(file);
                serde_json::from_reader(reader)
            })
            .await
            .unwrap()
            .unwrap_or_else(|err| panic!("Failed to deserialize file: {:?}\n Err: {err}", path,));
            data.user_id = *user_id;
            Some(data)
        } else {
            None
        }
    }

    /// save player data
    pub async fn save(&self) {
        let path = Self::get_path(&self.user_id);
        // Create the parent directories if they don't exist
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.unwrap();
        }

        let mut file = tokio::fs::File::create(&path)
            .await
            .unwrap_or_else(|err| panic!("failed to create file: {:?}, Err: {err}", path));
        let json_bytes = serde_json::to_vec(&self).unwrap_or_else(|err| {
            panic!(
                "Failed to serialize struct: {:?} to JSON. Err: {err}",
                std::any::type_name::<Self>(),
            )
        });
        if let Err(err) = file.write_all(&json_bytes).await {
            panic!("Failed to write JSON to file: {:?}. Err: {err}", path,);
        }
    }

    /// get user's file path
    fn get_path(user_id: &UserId) -> PathBuf {
        let mut path = PathBuf::from("./web_src/cache/user_search_history/");
        path.push(format!("{}.json", user_id));
        path
    }
}
