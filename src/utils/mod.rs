mod ship;
pub use ship::*;

use std::path::Path;

use poise::async_trait;
use serde::de::DeserializeOwned;

#[async_trait]
pub trait LoadFromJson {
    async fn load_json<P>(path: P) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
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
                std::io::BufReader::new(std::fs::File::open(path).expect("Failed to open file"));
            serde_json::from_reader(reader).expect("Failed to deserialize")
        })
        .await?;
        Ok(result)
        // let contents = fs::read(path).await?;
        // let data = serde_json::from_slice(&contents)?;
    }
}
