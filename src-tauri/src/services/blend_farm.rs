use crate::models::{
    message::{NetEvent, NetworkError},
    network::NetworkController,
};
use async_trait::async_trait;
use tokio::sync::mpsc::Receiver;

#[async_trait]
pub trait BlendFarm {
    async fn run(
        &self,
        client: NetworkController,
        event_receiver: Receiver<NetEvent>,
    ) -> Result<(), NetworkError>;
}
