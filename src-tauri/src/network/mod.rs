use std::{/*hash::DefaultHasher,*/ time::Duration};
use crate::{constant::TRANSFER, models::behaviour::BlendFarmBehaviour, network::{controller::Controller, message::{Command, Event, NetworkError}, service::Service}};
use libp2p::{gossipsub, identity, kad, mdns, noise, tcp, yamux, StreamProtocol, SwarmBuilder};
use libp2p_request_response::ProtocolSupport;
use machine_info::Machine;
use tokio::{io, sync::mpsc::{self, Receiver}};
pub(crate) mod provider_rule;
pub mod message;
pub mod network;
pub mod controller;
pub mod service;

// type is locally contained
pub type PeerIdString = String;

// the tuples return two objects
// Network Controller to interface network service
// Receiver<NetCommand> receive network events
pub async fn new(secret_key_seed:Option<u8>) -> Result<(Controller, Receiver<Event>, Service), NetworkError> {
    // wonder why we have a connection timeout of 60 seconds? Why not uint::MAX?

    let duration = Duration::from_secs(60);
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

            let rr_config = libp2p_request_response::Config::default();
            // Learn more about this and see if we need the transfer keyword of some sort?
            let protocol = [(StreamProtocol::new(TRANSFER), ProtocolSupport::Full)];
            let request_response = libp2p_request_response::Behaviour::new(protocol, rr_config);

            Ok(BlendFarmBehaviour {
                request_response,
                gossipsub,
                mdns,
                kademlia: kad,
            })
        })
        // TODO remove/handle expect()
        .expect("Expect to build behaviour")
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(duration))
        .build();

    // set the kad as server mode
    swarm.behaviour_mut().kademlia.set_mode(Some(kad::Mode::Server));

    // the command sender is used for outside method to send message commands to network queue
    let (sender, receiver) = mpsc::channel::<Command>(32);

    // the event sender is used to handle incoming network message. E.g. RunJob
    let (event_sender, event_receiver) = mpsc::channel::<Event>(32);

    let public_id = swarm.local_peer_id().clone();

    let controller = Controller::new(
        sender,
        public_id,
        Machine::new().system_info().hostname,
    );

    let service = Service::new(
        swarm,
        receiver,
        event_sender, // Here is where network service communicates out.
    );

    Ok((controller, event_receiver, service))
}

