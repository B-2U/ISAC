use std::{fs, io::Write, path::Path};

use serde::{Serialize, de::DeserializeOwned};
use tracing::{error, warn};

/// load and save the struct with given json path
///
/// ## Panic
/// panic when failing to load from the path
pub trait LoadSaveFromJson: Serialize + DeserializeOwned {
    const PATH: &'static str;

    async fn load_json() -> Self
    where
        Self: Default + Sized + Send + 'static,
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
        Self: Default,
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
        Self: Sized,
    {
        let json_bytes = serde_json::to_vec(&self).unwrap_or_else(|err| {
            panic!(
                "Failed to serialize struct: {:?} to JSON. Err: {err}",
                std::any::type_name::<Self>(),
            )
        });
        tokio::task::spawn_blocking(move || {
            save_file_with_lock(Self::PATH, &json_bytes);
        })
        .await
        .unwrap_or_else(|err| {
            panic!(
                "Failed to join async save_json for file: {:?}. Err: {err}",
                Self::PATH
            )
        });
    }

    fn save_json_sync(&self) {
        let json_bytes = serde_json::to_vec(&self).unwrap_or_else(|err| {
            panic!(
                "Failed to serialize struct: {:?} to JSON. Err: {err}",
                std::any::type_name::<Self>(),
            )
        });
        save_file_with_lock(Self::PATH, &json_bytes);
    }
}

/// Saves the provided data to the specified file path with file locking.
///
/// Ensures that the parent directory exists before writing. Locks the file for exclusive access
/// during the write operation to prevent concurrent writes.
///
/// # Arguments
/// * `path` - The file path to write to.
/// * `data` - The byte slice to write.
///
/// # Panics
/// - If creating directories, creating the file, or writing fails.
///
/// # Errors
/// - Logs an error if file locking fails, but does not panic.
pub fn save_file_with_lock(path: impl AsRef<Path>, data: &[u8]) {
    let path = path.as_ref();
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut file = std::fs::File::create(path)
        .unwrap_or_else(|err| panic!("failed to create file: {path:?}, Err: {err}"));
    if let Err(err) = file.lock() {
        error!("Failed to lock file: {path:?} for writing. Err: {err}");
        return;
    }
    if let Err(err) = file.write_all(data) {
        panic!("Failed to write JSON to file: {path:?}, Err: {err}");
    }
}
