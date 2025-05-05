use crate::models::{
        message::{Event, NetworkError},
        network::NetworkController,
    };
use async_trait::async_trait;
use futures::channel::mpsc::Receiver;

#[async_trait]
pub trait BlendFarm {
    async fn run(
        mut self,
        client: NetworkController,
        event_receiver: Receiver<Event>,
    ) -> Result<(), NetworkError>;
}
