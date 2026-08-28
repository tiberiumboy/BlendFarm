// use std::fmt::Display;
use std::io::Error as IoError;

use crate::domains::ticket_store::TicketError;
use crate::network::FileData;
// use crate::models::behaviour::FileResponse;
use crate::network::client::Client as NetworkController;
use crate::network::event::Event;
use crate::services::file_service::FileService;
// use crate::network::file_response::FileResponse;
// use crate::network::event::Event;
// FileCommand
use crate::network::message::NetworkError;
// use crate::services::file_service::FileCommand;
use async_trait::async_trait;
use blender_rs::blender::BlenderError;
use futures::Stream;
// use futures::Stream;
// use futures::FutureExt;
use futures::channel::mpsc::Receiver;
use futures::channel::oneshot;
use libp2p_request_response::ResponseChannel;
// use libp2p_request_response::ResponseChannel;

#[derive(Debug)]
pub enum BlendFarmError {
    Ticket(TicketError),
    Blender(BlenderError),
    Network(NetworkError),
    Io(IoError),
}

// impl Display for BlendFarmError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             BlendFarmError::Ticket(ticket_error) => {
//                 f.write_str(&format!("Received Ticket error: {ticket_error:?}"))
//             }
//             BlendFarmError::Network(network_error) => {
//                 f.write_str(&format!("Received network error: {network_error:?}"))
//             }
//             BlendFarmError::Io(error) => f.write_str(&format!("Received Io Error: {error:?}")),
//             BlendFarmError::Blender(error) => f.write_str(&error.to_string())
//         }
//     }
// }

/*

        let file_service = FileService::new();

        // once we received the data signal - process the remaining with the information obtained.
        if let Some(path) = file_service.get_file_path(&request) {
            // TODO: Remove/handle unwrap()
            let file = std::fs::read(path).unwrap();
            client.respond_file(file, channel).await;
        } else {
            eprintln!(
                "This local service does not have any matching request providing! Do something about the ResponseChannel?"
            );
        }

*/

#[async_trait]
pub trait BlendFarm {
    // TODO: Return mpsc stream for event notifications and system relays.
    async fn run(
        mut self,
        client: NetworkController,
        event_stream: impl Stream<Item = Event>,
        // event_receiver: Receiver<Event>,
        // TODO: Maybe consider returning the event from calling run()?
    ) -> Result<(), BlendFarmError>;

    // async fn handle_get_file(
    //     client: &mut NetworkController,
    //     file_name: &str,
    //     destination: &PathBuf,
    // ) -> Result<PathBuf, BlendFarmError> {
    //     let providers = client.get_providers(file_name.to_string()).await;
    //     let file_path = destination.join(file_name);

    //     let requests = providers.into_iter().map(|p| {
    //         let mut network_client = client.clone();
    //         async move { network_client.request_file(p, file_name.to_string()).await }.boxed()
    //     });

    //     let file_content = futures::future::select_ok(requests)
    //         .await
    //         .map_err(|_| BlendFarmError::NetworkError(NetworkError::NoPeerProviderFound))?
    //         .0;

    //     async_std::fs::write(file_path.clone(), file_content)
    //         .await
    //         .map_err(BlendFarmError::IoError)?;
    //     Ok(file_path)
    // }
}
