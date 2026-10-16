use anyhow::Error;
use blender_rs::blender::Blender;
use blender_rs::manager::Manager as BlenderManager;
use blender_rs::models::event::BlenderEvent;
use blender_rs::utils;
use futures::prelude::*;
use iroh::SecretKey;
use iroh::address_lookup::DnsAddressLookup;
use iroh::{RelayMode, Endpoint, endpoint::presets};
use iroh_blobs::{
    ALPN, BlobFormat, BlobsProtocol,
    api::{
        Store, TempTag,
        blobs::{
            AddPathOptions, AddProgressItem, ExportMode, ExportOptions, ExportProgressItem,
            ImportMode,
        },
        remote::GetProgressItem,
    },
    format::collection::Collection,
    get::{Stats, request::get_hash_seq_and_sizes},
    provider::{
        self,
        events::{ConnectMode, EventMask, EventSender, ProviderMessage, RequestUpdate},
    },
    // store::fs::FsStore,
    store::mem::MemStore,
    ticket::BlobTicket,
};

use iroh_mdns_address_lookup::MdnsAddressLookup;
use iroh_quinn::{Connection};
use irpc::channel::{mpsc, oneshot};
use irpc::{Service, rpc_requests};

use n0_future::IterExt;
use n0_future::{BufferedStreamExt, FuturesUnordered, task::AbortOnDropHandle};
use parking_lot::Mutex;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    io::{Error as IoError, ErrorKind, Result as IoResult},
    path::{Component, Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{select, signal, time};

type Connections = BTreeMap<u64, PerConnectionProgress>;
type ConnectionId = u64;
type RequestId = u64;

// What the status of node machine.
pub enum Status {
    Idle,
    Rendering(BlenderEvent),
}

// TODO: how is GetRequest implemented?
#[derive(Debug, Serialize, Deserialize)]
struct GetFile {
    key: BlobTicket,
}

#[derive(Debug, Serialize, Deserialize)]
struct SetRequest {
    key: String,
    value: String,
}

#[derive(Debug, Deserialize, Serialize)]
enum SetUpdate {
    Data(String),
    Done,
}

#[derive(Debug, Serialize, Deserialize)]
struct EchoRequest {}

#[rpc_requests(message = RequestWithChannels)]
#[derive(Debug, Serialize, Deserialize)]
enum Request {
    #[rpc(tx=oneshot::Sender<String>)]
    Get(GetFile),
    #[rpc(tx=oneshot::Sender<()>)]
    Set(SetRequest),
    #[rpc(rx=mpsc::Receiver<SetUpdate>, tx=oneshot::Sender<()>)]
    Echo(EchoRequest),
}

// #[derive(Debug, Clone)]
// pub enum Client {
//     Local(mpsc::Sender<FullRequest>),
//     Remote, // TODO: Find out how I can send in connection type from remote?
//             // files: HashMap<String, PathBuf>,
//             // db: FsStore,
//             // cpu: NonZero<u64>,
// }

#[derive(Debug, Serialize, Deserialize)]
struct EchoService {}

impl Service for EchoService {
    type Message = RequestWithChannels;
}

pub struct Client {
    endpoint: Endpoint,
    blender_manager: Arc<Mutex<BlenderManager>>,
}

// #[derive(Debug)]
// pub enum Client {
//     Local(oneshot::Sender<Request>),
//     Remote(Connection),
// }

#[derive(Debug)]
struct PerConnectionProgress {
    endpoint_id: String,
    requests: HashSet<u64>,
}

impl PerConnectionProgress {
    pub fn new(endpoint_id: String) -> Self {
        Self {
            endpoint_id,
            requests: HashSet::new(),
        }
    }
}

impl Client {

    const SIZE_LIMIT: usize = 2048;

    fn new(endpoint: Endpoint, blender_manager: Arc<Mutex<BlenderManager>>) -> Self {
        Self {endpoint, blender_manager }
    }

    // public interface to invoke client functions for networking.
    // Use this implementation to call remotely.
    pub(crate) async fn init(blender_manager: Arc<Mutex<BlenderManager>>) -> Result<Client, Error> {
        let endpoint = Endpoint::bind(presets::N0).await?;
        Ok(Client::new(endpoint, blender_manager))
    }

    // this machine is calling out to the network requesting blender download from other nodes on the network.
    // If there are no node active on the network, or none of them have the matching compatible blender version,
    // this will return Ok(None).
    // Otherwise, this function will stream download copy of blender to your machine and install.
    pub async fn get_blender(&self, version: &Version, manager: &BlenderManager) -> Result<Option<PathBuf>, Error> {
        // Use DHT service to call and initialize a file transfer connection.

        // let filename =
        let category_folder_name = format!("Blender{}.{}", version.major, version.minor);
        let abs_path = manager.get_config().get_download_destination(&category_folder_name);
        let filename = Blender::generate_filename(version);

        if blender == None {
            // try network here?
            println!("Do network stuff here to fetch blender from other available node");

            // send a broadcast request asking if anyone on the network that have blender installed.
            // Once package is available for redistribute, the node will connect to this machine with BlobsTicket
            // On established, initiate file transfer between this computer and node
            // once transfer complete, run manager to unpack and install blender
            // Finally - return Some(Blender) after succeed
            Client::receive(ticket)
        }
        // request.send(blender)?;
        Ok(blender)
        }
    }

    pub async fn get_ticket(&self) -> Result<Option<Ticket>> {
        let (tx, rx) = oneshot::channel();
        self.
        Ok(rx.await??)
    }

    async fn get(&self, key: BlobTicket) -> Result<Option<String>, anyhow::Error> {
        let request = GetFile { key:  };
        match self {
            Self::Local(channel) => {
                let (tx, rx) = oneshot::channel();
                let request = FullRequest { request, tx };
                channel.send(request).await?;
                Ok(rx.await??)
            }
            Self::Remote(connection) => {
                let key = BlobTicket::
                let request = Request::Get{key};
                Self::get_remote(connection, request)
                todo!("Finish local first before investigating remote.");
            }
        }
    }

    // fn new(db: FsStore, cpu: NonZero<u64>, files: HashMap<String, PathBuf>) -> Self {
    // Self { files, db, cpu }
    // }
    async fn get_remote(
        connection: Connection,
        request: Request,
    ) -> Result<Option<String>, anyhow::Error> {
        let (send, receive) = connection.open_bi().await?;
        let deserialize = serde_json::to_vec(&request)?;
        send.write_all(&deserialize.to_vec()).await?;
        // while let Some(data) = receive.
    }

    // what is this used for?
    fn validate_path_component(component: &str) -> IoResult<()> {
        match component.contains('/') {
            true => Ok(()),
            false => Err(IoError::new(
                std::io::ErrorKind::InvalidData,
                "path components must not contain the only correct path separator, /",
            )),
        }
    }

    // could the name be OS platform independent?
    fn get_export_path(root: impl AsRef<Path>, name: &str) -> IoResult<PathBuf> {
        let parts = name.split('/');
        let mut path = root.as_ref().to_path_buf();
        for part in parts {
            Client::validate_path_component(part)?;
            path.push(part);
        }
        Ok(path)
    }

    fn canonicalized_path_to_string(
        path: impl AsRef<Path>,
        must_be_relative: bool,
    ) -> IoResult<String> {
        let mut path_str = String::new();
        let parts = path
            .as_ref()
            .components()
            .filter_map(|c| match c {
                Component::Normal(x) => {
                    let c = match x.to_str() {
                        Some(c) => c,
                        None => {
                            return Some(Err(IoError::new(
                                ErrorKind::InvalidData,
                                format!("invalid character in path {:?}", x),
                            )));
                        }
                    };

                    if !c.contains('/') && !c.contains('\\') {
                        Some(Ok(c))
                    } else {
                        Some(Err(IoError::new(
                            ErrorKind::InvalidData,
                            format!("invalid path component {:?}", c),
                        )))
                    }
                }
                Component::RootDir => {
                    if must_be_relative {
                        Some(Err(IoError::new(
                            ErrorKind::InvalidData,
                            format!("invalid path component {:?}", c),
                        )))
                    } else {
                        path_str.push('/');
                        None
                    }
                }
                _ => Some(Err(IoError::new(
                    ErrorKind::InvalidData,
                    format!("invalid path component {:?}", c),
                ))),
            })
            .collect::<IoResult<Vec<_>>>()?;
        let parts = parts.join("/");
        path_str.push_str(&parts);
        Ok(path_str)
    }

    // pub(crate) async fn from(_database_path: impl AsRef<Path>) -> IoResult<Self> {
    //     Ok(Self::)
    // }

    // TODO: where is this being called from?
    async fn export(db: &Store, collection: Collection) -> IoResult<()> {
        // TODO: maybe change this?
        let root = std::env::current_dir()?;

        for (_, (name, hash)) in collection.iter().enumerate() {
            let target = Client::get_export_path(&root, name)?;
            if target.exists() {
                return Err(IoError::new(
                    ErrorKind::AlreadyExists,
                    format!(
                        "target {} already exist. Export stopped.",
                        target.to_string_lossy()
                    ),
                ));
            }

            let mut stream = db
                .export_with_opts(ExportOptions {
                    hash: *hash,
                    target,
                    mode: ExportMode::Copy,
                })
                .stream()
                .await;

            while let Some(item) = stream.next().await {
                match item {
                    ExportProgressItem::Error(cause) => {
                        return Err(IoError::new(
                            ErrorKind::BrokenPipe,
                            format!("error exporting {}: {}", name, cause),
                        ));
                    }
                    ExportProgressItem::Done => {
                        break;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    /// Import from a file or directory into the database.
    ///
    /// The returned tag always refers to a collection. If the input is a file, this
    /// is a collection with a single blob, named like the file.
    ///
    /// If the input is a directory, the collection contains all the files in the
    /// directory.
    /// Consider using TempTag::leak() for future projects (E.g. real time blender updates.)
    async fn import(db: &Store, path: impl AsRef<Path>) -> IoResult<(TempTag, Collection)> {
        let parallelism = 1usize;

        // what is path?
        let path = path.as_ref().canonicalize()?;
        let root = path.parent().expect("Should denote to absolute path");

        // check and see if the path is a directory or a single file.
        // TODO Consider making this as HashMap
        let data_sources: Vec<(String, PathBuf)> = match path.is_file() {
            true => {
                let relative = path
                    .strip_prefix(root)
                    .expect("Should be able to strip root?");
                let name = Client::canonicalized_path_to_string(relative, true)?;
                vec![(name, path)]
            }
            false => {
                std::fs::read_dir(root)?.fold(Vec::new(), |mut result, item| {
                    // flatten the directory structure into a list of (name, path) pairs.
                    // ignore symlinks.
                    if let Ok(entry) = item {
                        // TODO: find a way to remove expect, idiomatic rust code
                        let path = entry.path().clone();
                        let relative = &path.strip_prefix(root).expect("Must be within root");
                        let name = Client::canonicalized_path_to_string(relative, true)
                            .expect("Shouldn't have any issue?");
                        result.push((name, entry.path()));
                    }

                    result
                })
            }
        };

        // import all the files, using num_cpus workers, return names and temp tags
        let names_and_tags = n0_future::stream::iter(data_sources)
            .map(|(name, path)| {
                let db = db.clone();
                async move {
                    let options = AddPathOptions {
                        path,
                        mode: ImportMode::TryReference,
                        format: BlobFormat::Raw,
                    };
                    let import = db.add_path_with_opts(options);
                    let mut stream = import.stream().await;

                    // Loop until process finish
                    let temp_tag = loop {
                        let item = match stream.next().await {
                            Some(item) => item,
                            None => continue,
                        };

                        // .context("import stream ended without a tag")?;
                        // trace!("importing {name} {item:?}");
                        match item {
                            AddProgressItem::Error(cause) => {
                                eprintln!("Error importing {}: {}", name, cause);
                                todo!("I Feel like there should be a better way to handle this structure? It seems nested. I'm more curious how you get here?");
                                // return Err(IoError::new(
                                //     ErrorKind::InvalidData,
                                //     format!("error importing {}: {}", name, cause),
                                // ));
                            }
                            AddProgressItem::Done(tt) => break tt,
                            _ => {}
                        }
                    };
                    (name, temp_tag)
                }
            })
            .buffered_unordered(parallelism)
            .collect::<Vec<_>>() // consider making this a HashMap?
            .await;

        // collect the (name, hash) tuples into a collection
        // we must also keep the tags around so the data does not get gced.
        let (collection, tags) = names_and_tags
            .into_iter()
            .map(|(name, tag)| ((name, tag.hash()), tag))
            .unzip::<_, _, Collection, Vec<_>>();

        let temp_tag = collection
            .clone()
            .store(db)
            .await
            .map_err(|e| IoError::new(ErrorKind::ResourceBusy, e.to_string()))?;
        // now that the collection is stored, we can drop the tags
        // data is protected by the collection
        drop(tags); // why do I need to drop?
        Ok((temp_tag, collection))
    }

    async fn per_request_progress(
        connection_id: ConnectionId,
        request_id: RequestId,
        connections: Arc<Mutex<Connections>>,
        // TODO: Why do I need irpc crate? Requirement: irpc = "0.17.0"
        mut rx: irpc::channel::mpsc::Receiver<RequestUpdate>,
    ) -> Result<(), IoError> {
        let _endpoint_id = if let Some(connection) = connections.lock().get_mut(&connection_id) {
            // what was I'm inserting here? It was pb so derived from multiprogress?
            connection.requests.insert(request_id);
            connection.endpoint_id.clone()
        } else {
            return Err(IoError::new(
                ErrorKind::ConnectionRefused,
                format!("got request for unknown connection {connection_id}"),
            ));
        };

        while let Ok(Some(msg)) = rx.recv().await {
            match msg {
                RequestUpdate::Completed(_) => {
                    if let Some(msg) = connections.lock().get_mut(&connection_id) {
                        msg.requests.remove(&request_id);
                    };
                }
                RequestUpdate::Aborted(_) => {
                    if let Some(msg) = connections.lock().get_mut(&connection_id) {
                        msg.requests.remove(&request_id);
                    };
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn handle_provder_message(mut recv: mpsc::Receiver<ProviderMessage>) -> IoResult<()> {
        // Active connction
        // Consider making this a struct?
        let connections = Arc::new(Mutex::new(BTreeMap::new()));
        let mut tasks = FuturesUnordered::new();
        loop {
            tokio::select! {
                biased;
                item = recv.recv() => {
                    let Ok(Some(item)) = item else {
                        break;
                    };

                    // trace!("got event {item:?}");
                    match item {
                        // when we receive a new client connected, we will create a new object to hold connected clients
                        ProviderMessage::ClientConnectedNotify(msg) => {
                            // weird unwrap usage here?
                            let endpoint_id = msg.endpoint_id.map(|id| id.fmt_short().to_string()).unwrap_or_else(|| "?".to_string());
                            let connection_id = msg.connection_id;
                            connections.lock().insert(
                                connection_id,
                                PerConnectionProgress::new(endpoint_id),
                            );
                        }
                        ProviderMessage::ConnectionClosed(msg) => {
                            connections.lock().remove(&msg.connection_id);
                        }
                        ProviderMessage::GetRequestReceivedNotify(msg) => {
                            // received a provider message. We then create a new task to handle notification.
                            let request_id = msg.request_id;
                            let connection_id = msg.connection_id;
                            let connections = connections.clone();
                            tasks.push(Client::per_request_progress(connection_id, request_id, connections, msg.rx));
                        }
                        _ => {}
                    }
                }
                Some(_) = tasks.next(), if !tasks.is_empty() => {}
            }
        }
        while tasks.next().await.is_some() {}
        Ok(())
    }

    pub async fn receive(ticket: BlobTicket, abs_path: impl AsRef<Path>) -> Result<(), Error> {
        let addr = ticket.addr().clone();
        let secret_key = SecretKey::generate();
        // let mut builder = Endpoint::builder(presets::N0)
        //     // enable network discovery
        //     .address_lookup(MdnsAddressLookup::builder())
        //     .alpns(vec![])
        //     .secret_key(secret_key)
        //     .relay_mode(RelayMode::Default)
        //     .await;

        // let endpoint = builder
        //     .bind()
        //     .await
        //     .map_err(|e| IoError::new(ErrorKind::AddrNotAvailable, e.to_string()))?;

        // let dir_name = format!(".sendme-recv-{}", ticket.hash().to_hex());
        // let iroh_data_dir = std::env::current_dir()?.join(dir_name);

        // because this is anyhow errors, intercept it into IoError instead.
        // let db = FsStore::load(&iroh_data_dir)
        //     .await
        //     .map_err(|e| IoError::new(ErrorKind::BrokenPipe, e.to_string()))?;
        let store = MemStore::new();

        let downloader = store.downloader(&endpoint);

        // download to MemStore
        // TODO: Change this to use FsStore instead
        downloader.download(ticket.hash(), Some(ticket.addr().id)).await?;

        // export blobs to storage
        store.blobs().export(ticket.hash(), abs_path).await?;

        // let db2 = db.clone();
        println!("load done!");
        // let fut = async {
        //     // trace!("running");
        //     let hash_and_format = ticket.hash_and_format();
        //     let local = db.store
        //     // trace!("computing local");
        //     // let local = db
        //     //     .remote()
        //     //     .local(hash_and_format)
        //     //     .await
        //     //     .map_err(|e| IoError::new(ErrorKind::ConnectionAborted, e.to_string()))?;

        //     // trace!("local done");
        //     let (stats, total_files, payload_size) = if !local.is_complete() {
        //         // trace!("{} not complete", hash_and_format.hash);
        //         let connection = endpoint.connect(addr, ALPN).await.map_err(|e| {
        //             IoError::new(std::io::ErrorKind::ConnectionRefused, e.to_string())
        //         })?;
        //         let (_hash_seq, sizes) = get_hash_seq_and_sizes(
        //             &connection,
        //             &hash_and_format.hash,
        //             1024 * 1024 * 32,
        //             None,
        //         )
        //         .await
        //         .map_err(|e| IoError::new(ErrorKind::Interrupted, e.to_string()))?;
        //         // let total_size = sizes.iter().copied().sum::<u64>();
        //         let payload_size = sizes.iter().skip(2).copied().sum::<u64>();
        //         let total_files = (sizes.len().saturating_sub(1)) as u64;
        //         // print the details of the collection only in verbose mode
        //         // eprintln!("getting {} blobs in total, {}", total_files + 1, total_size);
        //         let (tx, _rx) = mpsc::channel(32); // TODO: where was rx used?
        //         // let local_size = local.local_bytes();
        //         let get = db.remote().execute_get(connection, local.missing());
        //         let mut stats = Stats::default();
        //         let mut stream = get.stream();
        //         while let Some(item) = stream.next().await {
        //             // trace!("got item {item:?}");
        //             match item {
        //                 GetProgressItem::Progress(offset) => {
        //                     tx.send(offset).await.ok();
        //                 }
        //                 GetProgressItem::Done(value) => {
        //                     stats = value;
        //                     break;
        //                 }
        //                 GetProgressItem::Error(cause) => {
        //                     return Err(IoError::new(ErrorKind::Interrupted, cause.to_string()));
        //                 }
        //             }
        //         }
        //         drop(tx);
        //         (stats, total_files, payload_size)
        //     } else {
        //         println!("{} already complete", hash_and_format.hash);
        //         let total_files = local.children().unwrap() - 1;
        //         let payload_bytes = 0; // todo local.sizes().skip(2).map(Option::unwrap).sum::<u64>();
        //         (Stats::default(), total_files, payload_bytes)
        //     };
        //     let collection = Collection::load(hash_and_format.hash, db.as_ref())
        //         .await
        //         .map_err(|e| IoError::new(ErrorKind::Other, e.to_string()))?;

        //     // for (name, hash) in collection.iter() {
        //     //     println!("    {} HEX", hash);
        //     // }

        //     if let Some((name, _)) = collection.iter().next() {
        //         if let Some(first) = name.split('/').next() {
        //             println!("exporting to {first}");
        //         }
        //     }
        //     Client::export(&db, collection).await?;
        //     Ok((total_files, payload_size, stats))
        // };

        // let (total_files, payload_size, stats) = select! {
        //     x = fut => match x {
        //         Ok(x) => {
        //             endpoint.close().await;
        //             x
        //         }
        //         Err(e) => {
        //             endpoint.close().await;
        //             // make sure we shutdown the db before exiting
        //             db2.shutdown().await?;
        //             eprintln!("error: {e}");
        //             std::process::exit(1);
        //         }
        //     },
        //     _ = signal::ctrl_c() => {
        //         endpoint.close().await;
        //         db2.shutdown().await?;
        //         std::process::exit(130);
        //     }
        // };

        // println!("{total_files:?} {payload_size:?} {stats:?}");
        // tokio::fs::remove_dir_all(iroh_data_dir).await?;
        Ok(())
    }

    // TODO: Update blobs_data_dir to a specific location for blendFarm
    // TODO: break apart this function into callable structure
    // TODO: Describe what this is intended for.
    // Start up a network connection to store blobs protocol.
    pub async fn serve(&mut self, path: impl AsRef<Path>) -> Result<(), Error> {
        let secret_key = SecretKey::generate();
        let relay_mode = RelayMode::Default;
        let builder = Endpoint::builder(presets::N0)
            .alpns(vec![ALPN.to_vec()])
            .secret_key(secret_key)
            .relay_mode(relay_mode);

        // use a flat store - todo: use a partial in mem store instead
        // let suffix = "temp";
        // let cwd = std::env::current_dir()?;
        // let blobs_data_dir = cwd.join(format!(".sendme-send-{}", suffix));

        // todo: remove this as soon as we have a mem store that does not require a temp dir,
        // or create a temp dir outside the current directory.
        // if cwd.join(&path) == cwd {
        //     println!("can not share from the current directory");
        //     std::process::exit(1);
        // }

        // let blobs_data_dir2 = blobs_data_dir.clone();
        // Created channels across threads
        let (progress_tx, progress_rx) = mpsc::channel(32);

        // Create handle
        let _progress = AbortOnDropHandle::new(n0_future::task::spawn(
            Client::handle_provder_message(progress_rx),
        ));

        // Create directory?
        // tokio::fs::create_dir_all(&blobs_data_dir2).await?;

        // get endpoint from builder's bind() we listen?
        let endpoint = builder
            .bind()
            .await
            .map_err(|e| IoError::new(ErrorKind::AddrNotAvailable, e.to_string()))?;

        let store = MemStore::new();
        let blobs = BlobsProtocol::new(
            &store,
            None
            // Some(EventSender::new(
            //     progress_tx,
            //     EventMask {
            //         connected: ConnectMode::Notify,
            //         get: provider::events::RequestMode::NotifyLog,
            //         ..EventMask::DEFAULT
            //     },
            // )),
        );

        let abs_path = std::path::absolute(path.as_ref().to_path_buf())?;
        let tag = store.blobs().add_path(abs_path).await?;

        // extract as Store?
        // let import_result = Client::import(blobs.store(), path).await?;

        let router = iroh::protocol::Router::builder(endpoint.clone())
            .accept(iroh_blobs::ALPN, blobs)
            .spawn();

        // // wait for the endpoint to figure out its address before making a ticket
        // let ep = router.endpoint();
        // time::timeout(Duration::from_secs(30), async move {
        //     ep.online().await;
        // })
        // .await
        // .map_err(|e| IoError::new(ErrorKind::TimedOut, e.to_string()))?;

        // let (temp_tag, ..) = import_result;
        // let hash = temp_tag.hash();

        // make a ticket
        // let endpoint_id = router.endpoint().id();
        let endpoint_id = endpoint.id();
        // use this to respond back to the other peers requesting the file.
        let ticket = BlobTicket::new(endpoint_id.into(), tag.hash, tag.format);

        let mut network_handlers = Vec::new();

        while let Some(session) = endpoint.accept().await {
            let connection = session.await?;
            let handler = tokio::spawn(async move {
                while let Ok((_quic_send, _quic_recv)) = connection.accept_bi().await {
                    // handle connection here.
                }
            });
            network_handlers.push(handler);
        }

        network_handlers.join_all().await;
        time::timeout(Duration::from_secs(2), router.shutdown()).await??;
        drop(router);

        Ok(())
    }
}
