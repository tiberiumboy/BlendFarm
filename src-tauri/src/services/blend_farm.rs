use std::path::PathBuf;

use crate::domains::ticket_store::TicketError;
use crate::models::behaviour::FileResponse;
use crate::network::controller::Controller as NetworkController;
use crate::network::message::{Event, FileCommand, NetworkError};
use tokio::sync::mpsc::Receiver;
use async_trait::async_trait;
use futures::FutureExt;
use futures::channel::oneshot;
use libp2p_request_response::ResponseChannel;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlendFarmError {
    // TODO: List out all possible error this program could produce.
    #[error("Ticket error: {0}")]
    TicketError(#[from] TicketError),
    #[error("Network error: {0}")]
    NetworkError(#[from] NetworkError),
    #[error("Io error: {0}")]
    IoError(#[from] std::io::Error)
}

#[async_trait]
pub trait BlendFarm {
    // TODO: Return mpsc stream for event notifications and system relays.
    async fn run(
        mut self,
        client: NetworkController,
        event_receiver: Receiver<Event>,
    ) -> Result<(), BlendFarmError>;

    // could we use this inside the blendfarm as a base class?
    async fn handle_inbound_request(
        client: &NetworkController,
        request: String,
        channel: ResponseChannel<FileResponse>,
    ) {
        let (sender, receiver) = oneshot::channel();
        let cmd = FileCommand::RequestFilePath {
            keyword: request,
            sender,
        };
        client.file_service(cmd).await;

        // once we received the data signal - process the remaining with the information obtained.
        if let Some(path) = receiver.await.expect("Sender should not be dropped") {
            // TODO: Remove/handle unwrap()
            let file = async_std::fs::read(path).await.unwrap();
            client.respond_file(file, channel).await;
        } else {
            eprintln!(
                "This local service does not have any matching request providing! Do something about the ResponseChannel?"
            );
        }
    }

    async fn handle_get_file(client: &mut NetworkController, file_name: &str, destination: &PathBuf) -> 
        Result<PathBuf, BlendFarmError> 
    {
        let providers = client.get_providers(&file_name).await;
        let file_path = destination.join(file_name);

        let requests = providers.into_iter().map(|p| {
            let mut network_client = client.clone();
            async move { network_client.request_file(&p, file_name).await }.boxed()
        });

        let file_content = futures::future::select_ok(requests).await.map_err(|_| NetworkError::NoPeerProviderFound)?
            .0;

        async_std::fs::write(file_path.clone(), file_content).await.map_err(BlendFarmError::IoError)?;
        Ok(file_path)
    }
}
