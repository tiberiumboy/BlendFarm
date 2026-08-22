use std::collections::HashSet;
use crate::{models::behaviour::FileResponse, network::message::FileData};
use crate::services::server::ServerEvent;
use crate::network::message::{Command, FileCommand, NetworkError};
use crate::network::provider_rule::ProviderRule;
use futures::channel::oneshot;
use libp2p::{Multiaddr, PeerId};
use libp2p_request_response::ResponseChannel;
use futures::channel::mpsc::Sender;
use futures::SinkExt;
// use futures::

//use tokio::sync::mpsc::error::SendError;

// Network Controller interfaces network service.
#[derive(Clone)]
pub struct Controller {
    sender: Sender<Command>, // send net commands
    // Should contain IPv4, TCP, and P2p protocol
    multiaddr: Multiaddr,
    // pub hostname: String,
}

impl Controller {
    pub(crate) fn new(sender: Sender<Command>, multiaddr: Multiaddr/* , hostname: String*/) -> Self {
        Self {
            sender,
            multiaddr,
            // hostname,
        }
    }   

    pub(crate) fn get_multiaddr(&self) -> &Multiaddr {
        &self.multiaddr
    }

    // pub(crate) async fn get_ticket(&mut self) -> Option<Ticket> {
    //     // let (sender, receiver) = oneshot::channel::<Ticket>();
    //     // todo: send a broadcast message out
    //     // when the host respond, we should get a reply back with information needed
        
    //     self.send_broadcast_message(RequestTicket()).await;

    //     // if let Ok(ticket) = receiver.await {
    //     // and expecting to wait for a receiver to return with a Option<Ticket> value.
    //     // return Some(ticket);
    //     // }
        
    //     None
    // }

    pub(crate) async fn start_listening(&mut self, addr: Multiaddr) {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::StartListening { addr, sender })
            .await
            .expect("Command receiver should never be dropped");
        if let Err(e) = receiver.await {
            eprintln!("Fail to listen? {e:?}");
        }
    }

    pub(crate) async fn subscribe(&mut self, topic: &str) -> () {
        // TODO: find a better way to get around to_owned(), but for now focus on getting this application to work.
        let cmd = Command::Subscribe {
            topic: topic.to_owned(),
        };
        
        if let Err(e) = self.sender.send(cmd).await {
            eprintln!("Unable to send command! {e:?}");
        }
    }

    // this is to broadcast message.
    pub(crate) async fn send_broadcast_message(&mut self, status: ServerEvent) {
        if let Err(e) = &self.sender.send(Command::Message(None, status)).await {
            eprintln!("Failed to send node status to network service: {e:?}");
        }
    }

    pub(crate) async fn send_peer_message(&mut self, peer_addr: &Multiaddr, status: ServerEvent) {
        if let Err(e) = &self.sender.send(Command::Message(Some(peer_addr.clone()), status)).await {
            eprintln!("Failed to send direct message to [{peer_addr}]! {e:?}");
        }
    }

    // TODO: Plan on using this call to dial peer for intracommunication protocol. 
    // pub(crate) async fn dial(
    //     &self,
    //     peer_addr: &Multiaddr,
    // ) -> Result<(), Box<dyn Error + Send>> {
    //     let (sender, receiver) = oneshot::channel();
    //     // we thread locked here. Awaiting for dial to come back successfully, which means we're establishing connection to exchange information.
    //     self.sender
    //         .send(Command::Dial {
    //             peer_addr: peer_addr.clone(),
    //             sender,
    //         })
    //         .await
    //         .expect("Should not drop");

    //     // so at this point we're waiting for connection established.
    //     if let Err(e) = receiver.await {
    //         eprintln!("Should not error? {e:?}");
    //     }

    //     println!("Connection established with [{peer_addr}]");
    //     Ok(())
    // }

    // received file service command.
    pub(crate) async fn file_service(&mut self, command: FileCommand) {
        if let Err(e) = self.sender
            .send(Command::FileService(command))
            .await {
                eprintln!("Command should not have been dropped! {e:?}");
            }
    }

    /// file_name are broadcasted with the extensions included, but not the directory it's located in. E.g. "test.blend"
    // I need to use some kind of enumeration to help make this process flexible with rules..
    pub(crate) async fn start_providing(
        &mut self,
        provider: &ProviderRule,
    ) -> Result<(), NetworkError> {
        let (sender, receiver) = oneshot::channel();
        let cmd = match provider {
            ProviderRule::Default(path_buf) => {
                // TODO: remove .expect(), .to_str(), and .to_owned()
                let file_name = path_buf.file_name().ok_or(NetworkError::BadInput)?;
                let keyword = file_name
                        .to_str()
                        .expect("Must be able to convert OsStr to Str!");

                FileCommand::StartProviding{ file_name: keyword.into(), sender }
            },
            // ProviderRule::Custom(keyword, .. ) => {
            //     FileCommand::StartProviding{ 
            //         file_name: keyword.to_owned(), 
            //         sender
            //     }
            // }
        };

        if let Err(e) = self.sender.send(Command::FileService(cmd)).await {
            eprintln!("How did this happen? {e:?}");
        }

        receiver.await.map_err(|e| NetworkError::SendError(e.to_string()))
    }

    pub async fn get_providers(&mut self, file_name: &str) -> HashSet<PeerId> {
        let (sender, receiver) = oneshot::channel();
        let cmd = Command::FileService(FileCommand::GetProviders {
            file_name: file_name.to_string(),
            sender,
        });
        match self.sender
            .send(cmd)
            .await
            {
                Err(e) => {
                    eprintln!("Unable to send internal message! {e:?}");
                    HashSet::new()
                }
                _ => receiver.await.unwrap_or(HashSet::new()),
            }
    }

    pub(crate) async fn request_file(
        &mut self,
        peer_id: &PeerId,
        file_name: &str,
    ) -> Result<FileData, NetworkError> {
        let (sender, receiver) = oneshot::channel();
        let file_command = FileCommand::RequestFile {
            peer_id: *peer_id,
            file_name: file_name.into(),
            sender,
        };
        self.sender
            .send(Command::FileService(file_command))
            .await
            .expect("Command should not be dropped");

        receiver
            .await
            .expect("Should not be closed?")
            .map_err(|e| NetworkError::UnableToSave(e.to_string()))
    }

    // TODO: Come back to this one and see how this one gets invoked.
    pub(crate) async fn respond_file(&mut self, file: Vec<u8>, channel: ResponseChannel<FileResponse>) {
        let cmd = Command::FileService(FileCommand::RespondFile { file, channel });
        if let Err(e) = self.sender.send(cmd).await {
            println!("Command should not be dropped: {e:?}");
        }
    }

    // Should send a notification to shutdown network services
    // TODO: impl this in drop or somewhere?
    pub async fn shutdown(mut self) {
        // closing sender connection should error out producer side to gracefully shutdown.
        self.sender.close_channel();
    }
}
