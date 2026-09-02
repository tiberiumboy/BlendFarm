use std::collections::HashMap;
use std::path::{Path, PathBuf};
use sqlx::{Pool, Sqlite};


///
/// The File service will be the middleware bridge between communicating SQL connection storage for a list of files we promote to provide on libp2p DHT entries.
/// When the program restart, we want to retain these records so that they would be available per request.
/// Things such as Blender softwares, rendered images, blend project files are listed here in this table.
pub(crate) struct FileService {
    providing_files: HashMap<String, PathBuf>,
}

impl FileService {
    pub fn new(db_conn: Pool<Sqlite>) -> Self {
        FileService {
            providing_files: HashMap::new(),
        }
    }

    pub fn get_file_path(&self, key: &str) -> Option<&PathBuf> {
        self.providing_files.get(key)
    }

    pub fn add_providing_file(&mut self, key: &str, path: impl AsRef<Path>) {
        self.providing_files
            .insert(key.to_owned(), path.as_ref().to_path_buf());
    }

    pub fn remove_providing_file(&mut self, key: &str) -> Option<PathBuf> {
        self.providing_files.remove(key)
    }
    /*
    async fn process_file_service(&mut self, cmd: FileCommand) {
        match cmd {
            /*
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

            */

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
                    .send_request(&peer_id, FileRequest::new(file_name.into()));
                self.pending_request_file.insert(request_id, sender);
            }
            FileCommand::RespondFile { file, channel } => {
                // somehow the send_response errored out? How come?
                // Seems like this function got timed out?
                if let Err(e) = self
                    .swarm
                    .behaviour_mut()
                    .request_response
                    .send_response(channel, FileResponse::new(file))
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
    */
}
