use crate::domains::ticket_store::TicketError;
use crate::models::behaviour::FileResponse;
use crate::network::controller::Controller as NetworkController;
use crate::network::message::{Event, FileCommand, NetworkError};
use async_trait::async_trait;
use futures::channel::oneshot;
use libp2p_request_response::ResponseChannel;
use thiserror::Error;
use tokio::sync::mpsc::Receiver;

#[derive(Debug, Error)]
pub enum BlendFarmError {
    // TODO: List out all possible error this program could produce.
    #[error("Ticket error: {0}")]
    TicketError(#[from] TicketError),
    #[error("Network error: {0}")]
    NetworkERror(#[from] NetworkError),
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
            let file = async_std::fs::read(path).await.unwrap();
            client.respond_file(file, channel).await;
        } else {
            eprintln!(
                "This local service does not have any matching request providing! Do something about the ResponseChannel?"
            );
        }
    }
}
