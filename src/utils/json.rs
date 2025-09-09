use std::{fs, path::Path};

use serde::{Serialize, de::DeserializeOwned};
use tokio::io::AsyncWriteExt;
use tracing::warn;

/// load and save the struct with given json path
///
/// ## Panic
/// panic when failing to load from the path
pub trait LoadSaveFromJson {
    const PATH: &'static str;
    async fn load_json() -> Self
    where
        Self: Default + Serialize + DeserializeOwned + Sized + Send + 'static,
    {
        tokio::task::spawn_blocking(move || {
            if let Ok(file) = std::fs::File::open(Self::PATH) {
                let json_str = std::io::read_to_string(file).unwrap();
                serde_json::from_str(&json_str).unwrap_or_else(|err| {
                    panic!(
                        "Failed to deserialize file: {:#?} to struct: {}\n Err: {err}",
                        Self::PATH,
                        std::any::type_name::<Self>()
                    )
                })
            } else {
                warn!(
                    "file: {:#?} wasn't existed, initializing a dafault one",
                    Self::PATH
                );
                let default = Self::default();
                default.save_json_sync();
                default
            }
        })
        .await
        .unwrap_or_else(|err| {
            panic!(
                "Failed to join async load_json for file: {:?}. Err: {err}",
                Self::PATH
            )
        })
    }

    fn load_json_sync() -> Self
    where
        Self: DeserializeOwned + Serialize + Default,
    {
        if let Ok(file) = std::fs::File::open(Self::PATH) {
            let json_str = std::io::read_to_string(file).unwrap();
            serde_json::from_str(&json_str).unwrap_or_else(|err| {
                panic!(
                    "Failed to deserialize file: {:?} to struct: {}\n Err: {err}",
                    Self::PATH,
                    std::any::type_name::<Self>()
                )
            })
        } else {
            warn!(
                "file: {} wasn't existed, initializing a dafault one",
                Self::PATH
            );
            let default = Self::default();
            default.save_json_sync();
            default
        }
    }

    async fn save_json(&self)
    where
        Self: Serialize + Sized,
    {
        // Create the parent directories if they don't exist
        if let Some(parent) = Path::new(Self::PATH).parent() {
            tokio::fs::create_dir_all(parent).await.unwrap();
        }
        let mut file = tokio::fs::File::create(&Self::PATH)
            .await
            .unwrap_or_else(|err| panic!("failed to create file: {:?}, Err: {err}", Self::PATH));
        let json_bytes = serde_json::to_vec(&self).unwrap_or_else(|err| {
            panic!(
                "Failed to serialize struct: {:?} to JSON. Err: {err}",
                std::any::type_name::<Self>(),
            )
        });

        if let Err(err) = file.write_all(&json_bytes).await {
            panic!("Failed to write JSON to file: {:?}. Err: {err}", Self::PATH,);
        }
        // serde_json::to_writer(file, &self).unwrap_or_else(|err| {
        //     panic!(
        //         "Failed to serialze struct: {:?} to file: {}\n Err: {err}",
        //         std::any::type_name::<Self>(),
        //         Self::PATH,
        //     )
        // })
    }

    fn save_json_sync(&self)
    where
        Self: Serialize,
    {
        if let Some(parent_dir) = std::path::Path::new(Self::PATH).parent() {
            fs::create_dir_all(parent_dir).unwrap();
        }
        let file = std::fs::File::create(Self::PATH)
            .unwrap_or_else(|err| panic!("failed to create file: {:?}, Err: {err}", Self::PATH));
        serde_json::to_writer(file, &self).unwrap_or_else(|err| {
            panic!(
                "Failed to serialze struct: {:?} to file: {}\n Err: {err}",
                std::any::type_name::<Self>(),
                Self::PATH,
            )
        })
    }
}
