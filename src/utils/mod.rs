pub mod structs;
pub mod wws_api;

mod isac_error;
pub use isac_error::{IsacError, IsacHelp, IsacInfo};

use std::path::Path;

use poise::async_trait;
use serde::de::DeserializeOwned;

#[async_trait]
pub trait LoadFromJson {
    async fn load_json<P>(path: P) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
    where
        Self: DeserializeOwned + Sized,
        P: AsRef<Path> + std::marker::Send;

    fn load_json_sync<P>(path: P) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
    where
        Self: DeserializeOwned + Sized,
        P: AsRef<Path> + std::marker::Send;
}
#[async_trait]
impl<O> LoadFromJson for O
where
    O: DeserializeOwned + Sized + Send + 'static,
{
    async fn load_json<P>(path: P) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
    where
        Self: DeserializeOwned + Sized,
        P: AsRef<Path> + std::marker::Send,
    {
        let path = path.as_ref().to_owned();
        let result = tokio::task::spawn_blocking(move || {
            let reader =
                std::io::BufReader::new(std::fs::File::open(&path).expect("Failed to open file"));
            serde_json::from_reader(reader).unwrap_or_else(|err| {
                panic!(
                    "Failed to deserialize file: {path:?} to struct: {}\n Err: {err}",
                    std::any::type_name::<Self>()
                )
            })
        })
        .await?;
        Ok(result)
    }

    fn load_json_sync<P>(path: P) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
    where
        Self: DeserializeOwned + Sized,
        P: AsRef<Path> + std::marker::Send,
    {
        let path = path.as_ref().to_owned();
        let reader =
            std::io::BufReader::new(std::fs::File::open(&path).expect("Failed to open file"));
        Ok(serde_json::from_reader(reader).unwrap_or_else(|err| {
            panic!(
                "Failed to deserialize file: {path:?} to struct: {}\n Err: {err}",
                std::any::type_name::<Self>()
            )
        }))
    }
}
