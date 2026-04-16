use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::models::behaviour::FileResponse;
use crate::services::server::ServerEvent;
use crate::network::message::{Command, FileCommand, NetworkError};
use crate::network::provider_rule::ProviderRule;
use std::error::Error;
use futures::channel::oneshot::{self};
use libp2p::{Multiaddr, PeerId};
use libp2p_request_response::ResponseChannel;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::SendError;

// Network Controller interfaces network service.
#[derive(Clone)]
pub struct Controller {
    sender: Sender<Command>, // send net commands
    pub multiaddr: Multiaddr,
    pub hostname: String,
}

impl Controller {
    pub(crate) fn new(sender: Sender<Command>, multiaddr: Multiaddr, hostname: String) -> Self {
        Self {
            sender,
            multiaddr,
            hostname,
        }
    }

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

    pub(crate) async fn subscribe(&mut self, topic: &str) -> Result<(), SendError<Command>> {
        // TODO: find a better way to get around to_owned(), but for now focus on getting this application to work.
        let cmd = Command::Subscribe {
            topic: topic.to_owned(),
        };
        self.sender.send(cmd).await
    }

    // this is to broadcast message.
    pub(crate) async fn send_broadcast_message(&self, status: ServerEvent) {
        if let Err(e) = self.sender.send(Command::Message(None, status)).await {
            eprintln!("Failed to send node status to network service: {e:?}");
        }
    }

    pub(crate) async fn send_peer_message(&self, peer_id: &PeerId, status: ServerEvent) {
        if let Err(e) = self.sender.send(Command::Message(Some(peer_id.clone()), status)).await {
            eprintln!("Failed to send direct message to [{peer_id}]! {e:?}");
        }
    }

    /* 
    #[allow(dead_code)]
    fn strip_peer_id(addr: &mut Multiaddr) {
        let last = addr.pop();
        match last {
            Some(Protocol::P2p(peer_id)) => {
                let mut addr = Multiaddr::empty();
                addr.push(Protocol::P2p(peer_id));
                println!("Removing peer id [{addr}] so this address can be dialed by rust-libp2p");
            }
            Some(other) => addr.push(other),
            _ => {}
        }
    }

    #[allow(dead_code)]
    fn parse_legacy_multiaddr(text: &str) -> Result<Multiaddr, Box<dyn Error>> {
        let sanitized = text
            .split('/')
            .map(|part| if part == "ipfs" { "p2p" } else { part })
            .collect::<Vec<_>>()
            .join("/");
        let mut res = Multiaddr::from_str(&sanitized)?;
        Self::strip_peer_id(&mut res);
        Ok(res)
    }
    */

    pub(crate) async fn dial(
        &self,
        peer_addr: &Multiaddr,
    ) -> Result<(), Box<dyn Error + Send>> {
        let (sender, receiver) = oneshot::channel();
        // we thread locked here. Awaiting for dial to come back successfully, which means we're establishing connection to exchange information.
        self.sender
            .send(Command::Dial {
                peer_addr: peer_addr.clone(),
                sender,
            })
            .await
            .expect("Should not drop");

        // so at this point we're waiting for connection established.
        if let Err(e) = receiver.await {
            eprintln!("Should not error? {e:?}");
        }

        println!("Connection established with [{peer_addr}]");
        Ok(())
    }

    // send job event to all connected node
    // pub async fn send_job_event(&self, event: JobEvent) {
    //     let server_event = ServerEvent::RemoveJob(())
    //     self.sender
    //             .send(Command::ServerStatus(event))
    //         .await
    //         .expect("Command should not be dropped");
    // }

    pub(crate) async fn file_service(&self, command: FileCommand) {
        self.sender
            .send(Command::FileService(command))
            .await
            .expect("Command should not have been dropped!");
    }

    /// file_name are broadcasted with the extensions included, but not the directory it's located in. E.g. "test.blend"
    // I need to use some kind of enumeration to help make this process flexible with rules..
    pub(crate) async fn start_providing(
        &self,
        provider: &ProviderRule,
    ) -> Result<(), NetworkError> {
        let cmd = match provider {
            ProviderRule::Default(path_buf) => {
                // TODO: remove .expect(), .to_str(), and .to_owned()
                match path_buf.file_name() {
                    Some(file_name) => {
                        let keyword = file_name
                            .to_str()
                            .expect("Must be able to convert OsStr to Str!");

                        FileCommand::StartProviding(keyword.into(), path_buf.into())
                    }
                    None => return Err(NetworkError::BadInput),
                }
            }
            ProviderRule::Custom(keyword, path_buf) => {
                FileCommand::StartProviding(keyword.to_owned(), path_buf.to_owned())
            }
        };

        if let Err(e) = self.sender.send(Command::FileService(cmd)).await {
            eprintln!("How did this happen? {e:?}");
        }
        Ok(())
    }

    pub async fn get_providers(&mut self, file_name: &str) -> HashSet<PeerId> {
        let (sender, receiver) = oneshot::channel();
        let cmd = Command::FileService(FileCommand::GetProviders {
            file_name: file_name.to_string(),
            sender,
        });
        self.sender
            .send(cmd)
            .await
            .expect("Command receiver should not be dropped");
        receiver.await.unwrap_or(HashSet::new())
    }

    // client request file from peers.
    // I feel like we should make this as fetching data from network? Some sort of stream?
    pub async fn get_file_from_peers<T: AsRef<Path>>(
        &mut self,
        file_name: &str,
        destination: T,
    ) -> Result<PathBuf, NetworkError> {
        let providers = self.get_providers(&file_name).await;
        match providers.iter().next() {
            Some(peer_id) => {
                self.request_file(peer_id, file_name, destination.as_ref())
                    .await
            }
            None => Err(NetworkError::NoPeerProviderFound),
        }
    }

    async fn request_file(
        &mut self,
        peer_id: &PeerId,
        file_name: &str,
        destination: &Path,
    ) -> Result<PathBuf, NetworkError> {
        let (sender, receiver) = oneshot::channel();
        let cmd = Command::FileService(FileCommand::RequestFile {
            peer_id: *peer_id,
            file_name: file_name.into(),
            sender,
        });
        self.sender
            .send(cmd)
            .await
            .expect("Command should not be dropped");
        let content = receiver
            .await
            .expect("Should not be closed?")
            .or_else(|e| Err(NetworkError::UnableToSave(e.to_string())))?;

        let file_path = destination.join(file_name);
        match async_std::fs::write(file_path.clone(), content).await {
            Ok(_) => Ok(file_path),
            Err(e) => Err(NetworkError::UnableToSave(e.to_string())),
        }
    }

    // TODO: Come back to this one and see how this one gets invoked.
    pub(crate) async fn respond_file(&self, file: Vec<u8>, channel: ResponseChannel<FileResponse>) {
        let cmd = Command::FileService(FileCommand::RespondFile { file, channel });
        if let Err(e) = self.sender.send(cmd).await {
            println!("Command should not be dropped: {e:?}");
        }
    }
}
