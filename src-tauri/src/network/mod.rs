use crate::network::client::Client;
// use futures::channel::mpsc::{self, Receiver}; // cbor, Config
use std::io::Result as IoResult;

pub(crate) mod client;
// pub mod service;

/// Creates the network components, namely:
///
/// - The network client to interact with the network layer from anywhere within your application.
///
/// - The network event stream, e.g. for incoming requests.
///
/// - The network task driving the network itself.
pub(crate) async fn new(
    _secret_key_seed: Option<u8>, // TODO: future implementation, but not for scope of this goal
) -> IoResult<
    //(
    Client, //, Receiver<Event>, EventLoop),
> {
    let database_path = blender_rs::utils::get_config_folder_path()?;
    let client = Client::from(database_path).await?;
    Ok(
        // (
        client,
        // event_receiver,
        // EventLoop::new(swarm, command_receiver, event_sender),
        // )
    )
}

/*

// the tuples return three objects
// Network Controller to interface network service
// Receiver<NetCommand> receive network events
// Service contains body instructions of Network infrastructure.  Must run on a separate thread
pub async fn new(
    secret_key_seed: Option<u8>,
) -> Result<(Controller, Receiver<Event>/*impl Stream<Item = Event>*/, Service), NetworkError> {
    // Maximum time allowed for established stream connections.
    let duration = Duration::from_secs(60);

    // port to allow connection
    let port = 8082;

    // max channel allowed
    let max_channel_buffer: usize = 8;

    // is there a reason for the secret key seed?
    let id_keys = match secret_key_seed {
        Some(seed) => {
            let mut bytes = [0u8; 32];
            bytes[0] = seed;
            identity::Keypair::ed25519_from_bytes(bytes).unwrap()
        }
        None => identity::Keypair::generate_ed25519(),
    };

    let mut swarm = SwarmBuilder::with_existing_identity(id_keys)
        // let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .expect("Should be able to build with tcp configuration?")
        .with_quic()
        .with_behaviour(|key| {
            // seems like we need to content-address message. We'll use the hash of the message as the ID.
            // let message_id_fn = |message: &gossipsub::Message| {
            //     let mut s = DefaultHasher::new();
            //     message.data.hash(&mut s);
            //     gossipsub::MessageId::from(s.finish().to_string())
            // };

            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(10))
                .validation_mode(gossipsub::ValidationMode::Strict)
                // .message_id_fn(message_id_fn)
                .build()
                .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))?;

            // p2p communication
            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )
            .expect("Fail to create gossipsub behaviour");

            // network discovery usage
            // TODO: replace expect with error handling
            let mdns =
                mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())
                    .expect("Fail to create mdns behaviour!");

            // Used to provide file provision list
            let kad = kad::Behaviour::new(
                key.public().to_peer_id(),
                kad::store::MemoryStore::new(key.public().to_peer_id()),
            );

        })
        // TODO remove/handle expect()
        .expect("Expect to build behaviour")
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(duration))
        .build();

    // set the kad as server mode
    swarm
        .behaviour_mut()
        .kademlia
        .set_mode(Some(kad::Mode::Server));

    // the command sender is used for outside method to send message commands to network queue
    let (sender, receiver) = mpsc::channel::<Command>(max_channel_buffer);

    // the event sender is used to handle incoming network message. E.g. RunJob
    let (event_sender, event_receiver) = mpsc::channel::<Event>(max_channel_buffer);

    let public_id = swarm.local_peer_id().clone();
    let mut multiaddr = Multiaddr::empty();
    // TODO: How do I get the network assigned address of this computer?
    multiaddr.push(Protocol::Ip4(Ipv4Addr::LOCALHOST));
    multiaddr.push(Protocol::Tcp(port));
    multiaddr.push(Protocol::P2p(public_id));

    let controller = Controller::new(sender, multiaddr);
    let service = Service::new(
        swarm,
        receiver,
        event_sender, // Here is where network service communicates out.
    );

    Ok((controller, event_receiver, service))
}
*/
