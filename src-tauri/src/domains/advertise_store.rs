use crate::models::advertise::Advertise;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum AdvertiseError {
    #[error("Unknown")]
    Unknown,
    #[error("Received Database errors! {0}")]
    DatabaseError(String),
}

#[async_trait::async_trait]
pub trait AdvertiseStore {
    async fn find(&self, id: Uuid) -> Result<Option<Advertise>, AdvertiseError>;
    async fn update(&self, advertise: Advertise) -> Result<(), AdvertiseError>;
    async fn create(&self, advertise: Advertise) -> Result<(), AdvertiseError>;
    async fn kill(&self, id: Uuid) -> Result<(), AdvertiseError>;
    async fn all(&self) -> Result<Option<Vec<Advertise>>, AdvertiseError>;
}
