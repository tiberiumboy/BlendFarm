use crate::models::behaviour::FileResponse;
use crate::network::controller::Controller as NetworkController;
use crate::network::message::{Event, FileCommand, NetworkError};
use async_trait::async_trait;
use futures::channel::oneshot;
use libp2p_request_response::ResponseChannel;
use tokio::sync::mpsc::Receiver;

#[async_trait]
pub trait BlendFarm {
    // TODO: Return mpsc stream for event notifications and system relays.
    async fn run(
        mut self,
        client: NetworkController,
        event_receiver: Receiver<Event>,
    ) -> Result<(), NetworkError>;

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
