use futures::channel::oneshot;
use libp2p::{Multiaddr, PeerId, multiaddr::Protocol};
use libp2p_request_response::ResponseChannel;
use std::collections::HashSet;
use std::{collections::hash_map, error::Error, path::PathBuf};

use crate::{
    models::behaviour::{FileRequest, FileResponse},
    network::message::KeywordSearch,
};

// TODO: Find a way to cast this as FileStruct?
pub type FileData = Vec<u8>;

// TODO: Find a way to handle errors properly
pub type FileResult<T> = Result<T, Box<dyn Error + Send>>;

// to make things simple, we'll create a file service command to handle file service.
#[derive(Debug)]
pub enum FileCommand {
    Dial {
        peer_addr: Multiaddr,
        sender: oneshot::Sender<FileResult<()>>,
    },
    // TODO: Find a way to get around the string type! This expects a copy!
    StartProviding {
        file_name: KeywordSearch,
        sender: oneshot::Sender<()>,
    },
    // StartProviding(KeywordSearch, PathBuf), // update kademlia service to provide a new file. Must have a file name and a extension! Cannot be a directory!
    StopProviding(KeywordSearch), // update kademlia service to stop providing the file.
    GetProviders {
        file_name: String,
        sender: oneshot::Sender<HashSet<PeerId>>,
    },
    RequestFile {
        peer_id: PeerId,
        file_name: String,
        sender: oneshot::Sender<FileResult<FileData>>,
    },
    RespondFile {
        file: FileData,
        channel: ResponseChannel<FileResponse>,
    },
    RequestFilePath {
        keyword: KeywordSearch,
        sender: oneshot::Sender<Option<PathBuf>>,
    },
}

/*
struct FileService {}

impl FileService {
    async fn process_file_service(&mut self, cmd: FileCommand) {
        match cmd {
            FileCommand::Dial {
                mut peer_addr,
                sender,
            } => {
                // Expect peer_id contain multiaddress otherwise return early
                let Some(Protocol::P2p(peer_id)) = peer_addr.pop() else {
                    println!(
                        "No peer id found in multi-address! skipping! Must include '.../p2p/peer_id'!"
                    );
                    return;
                };

                let hash_map::Entry::Vacant(e) = self.pending_dial.entry(peer_id) else {
                    // I would expect the multiaddr have peer_id attached.
                    // TODO: A bruteforce attempt could be made to break this system integrity. Consider rate limiting?
                    eprintln!("Already dialing the peer! Please be patient!");
                    return;
                };

                self.swarm
                    .behaviour_mut()
                    .kademlia
                    .add_address(&peer_id, peer_addr.clone());

                // The main reason why I need to dial this node is so I can
                //  1. Know which node I'm talking to.
                //  2. Distribute render files, blend files, and executables
                //  3. Performance monitor / Activity Logs
                match self.swarm.dial(peer_addr.with(Protocol::P2p(peer_id))) {
                    Ok(()) => {
                        e.insert(sender);
                    }
                    Err(e) => {
                        // TODO: handle expect gracefully.
                        sender.send(Err(Box::new(e))).expect("Should not drop");
                    }
                }
            }

            // use this to advertise files. On app startup we should broadcast blender apps as well.
            FileCommand::StartProviding { file_name, sender } => {
                // TODO: Find a way to get around expect()!
                let query_id = self
                    .swarm
                    .behaviour_mut()
                    .kademlia
                    .start_providing(file_name.into_bytes().into())
                    .expect("No store value");
                self.pending_start_providing.insert(query_id, sender);
            }

            FileCommand::StopProviding(file_name) => {
                let key = file_name.into_bytes();
                self.swarm
                    .behaviour_mut()
                    .kademlia
                    .stop_providing(&key.into());
                // TODO: I want to clear any pending providing, I need to find a way to fetch query ID before stop file providing.
                // self.pending_start_providing.remove_entry(&key);
            }
            FileCommand::RequestFile {
                peer_id,
                file_name,
                sender,
            } => {
                let request_id = self
                    .swarm
                    .behaviour_mut()
                    .request_response
                    .send_request(&peer_id, FileRequest(file_name.into()));
                self.pending_request_file.insert(request_id, sender);
            }
            FileCommand::RespondFile { file, channel } => {
                // somehow the send_response errored out? How come?
                // Seems like this function got timed out?
                if let Err(e) = self
                    .swarm
                    .behaviour_mut()
                    .request_response
                    .send_response(channel, FileResponse(file))
                {
                    // why am I'm getting error message here?
                    eprintln!("Error received on sending response! {e:?}");
                }
            }
            FileCommand::GetProviders { file_name, sender } => {
                let key = file_name.into_bytes().into();
                let query_id = self.swarm.behaviour_mut().kademlia.get_providers(key);
                self.pending_get_providers.insert(query_id, sender);
            }
            // FileCommand::StartProviding(keyword, file_path) => {
            //     let key = keyword.clone().into_bytes().into();
            //     // could we make use of this query ID?
            //     let _query_id = self
            //         .swarm
            //         .behaviour_mut()
            //         .kademlia
            //         .start_providing(key)
            //         .expect("No store error.");
            //     self.providing_files.insert(keyword, file_path);
            // }
            FileCommand::RequestFilePath { keyword, sender } => {
                let result = self
                    .providing_files
                    .get(&keyword)
                    .and_then(|f| Some(f.to_owned()));
                println!("{keyword:?} | {result:?}");
                sender.send(result).expect("Receiver should not be dropped");
            }
        };
    }
}
*/
