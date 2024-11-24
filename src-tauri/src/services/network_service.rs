use libp2p::futures::StreamExt;
use libp2p::gossipsub::{self, IdentTopic};
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{mdns, Multiaddr};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::mpsc::{self, Sender};
use tokio::task::JoinHandle;
use tokio::{io, select};

use crate::models::job::Job;

/*
the idea behind this is to have a persistence model to contain network services.
the netework services will be able to run either as a host or a node.
the network services will also handle all of the incoming network packages and process the stream

TODO: Find a way to send notification to Tauri application on network process message.
TODO: obtain basic computer specs (CPU/GPU/RAM/OS/Arch/Blender installed)

Resources:
    Administratively scoped IPv4 multicast space - https://datatracker.ietf.org/doc/html/rfc2365
*/

pub type Port = u16;

#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("Not Connected")]
    NotConnected,
    #[error("Invalid connection")]
    Invalid,
    #[error("Bad Input")]
    BadInput,
}

#[derive(NetworkBehaviour)]
pub struct BlendFarmBehaviour {
    // ping: libp2p::ping::Behaviour,
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
    // I want to include kad for file share protocol
    // kad: libp2p::kad::Behaviour<T>// Ok so I need to figure out how this works? Figure out about TStore trait
}

#[derive(Debug, Serialize, Deserialize)]
pub enum NetMessage {
    // Greet () // share machine spec (cpu, gpu, ram)
    // Heartbeat() // share hardware statistic monitor heartbeat. (CPU/GPU/RAM usage realtime)
    Render(Job),
    SendFile { file_name: String, data: Vec<u8> },
    Status(String),
}

impl NetMessage {
    pub fn ser(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn de(data: &[u8]) -> Result<Self, Box<bincode::ErrorKind>> {
        bincode::deserialize(data)
    }
}

// this will help launch libp2p network. Should use QUIC whenever possible!
pub struct NetworkService {
    tx: Sender<NetMessage>,
    _task: JoinHandle<()>,
}

impl NetworkService {
    // I think it's best if we take in common demoniator value as possible.
    pub async fn new(idle_connection_timeout: u64) -> Result<Self, Box<dyn std::error::Error>> {
        // try to parse the duration before usage.
        let duration = Duration::from_secs(idle_connection_timeout);
        // required by swarm
        let tcp_config: libp2p::tcp::Config = libp2p::tcp::Config::default();
        // TODO: Figure out what this one is suppose to do?
        let topic = IdentTopic::new("blendfarm-rpc-msg");

        let mut swarm = libp2p::SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                tcp_config,
                libp2p::tls::Config::new,
                libp2p::yamux::Config::default,
            )?
            .with_quic()
            .with_behaviour(|key| {
                let message_id_fn = |message: &gossipsub::Message| {
                    let mut s = DefaultHasher::new();
                    message.data.hash(&mut s); // what was this suppose to do?
                    gossipsub::MessageId::from(s.finish().to_string())
                };

                let gossipsub_config = gossipsub::ConfigBuilder::default()
                    .heartbeat_interval(Duration::from_secs(10))
                    .validation_mode(gossipsub::ValidationMode::Strict)
                    .message_id_fn(message_id_fn)
                    .build()
                    .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))?;

                let gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub_config,
                )
                .expect("Fail to create gossipsub behaviour");

                let mdns =
                    mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())
                        .expect("Fail to create mdns behaviour!");

                Ok(BlendFarmBehaviour { gossipsub, mdns })
            })?
            // .with_behaviour(|_| dummy::Behaviour)?
            .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(duration))
            .build();

        swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

        // TODO: Find a way to fetch user configuration. Refactor this when possible.
        let udp: Multiaddr = "/ip4/0.0.0.0/udp/0/quic-v1"
            .parse()
            .expect("Must be valid multiaddr");
        let tcp: Multiaddr = "/ip4/0.0.0.0/tcp/0"
            .parse()
            .expect("Must be valid multiaddr");
        swarm.listen_on(tcp)?;
        swarm.listen_on(udp)?;

        // println!("About to dial as random!");
        // Problem here - I dont know why this would return NoAddresses? Should this be something else or how do I send out ping?
        // swarm.dial(PeerId::random())?;

        // create a new channel with a capacity of at most 32.
        let (tx, mut rx) = mpsc::channel::<NetMessage>(32);

        // create a thread here?
        let _task = tokio::spawn(async move {
            loop {
                select! {
                    // Sender
                    Some(signal) = rx.recv() => {
                        let topic = gossipsub::IdentTopic::new("blendfarm-rpc-msg");
                        let data = signal.ser();
                        if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, data) {
                            println!("Fail to publish message to swarm! {e:?}");
                        }
                    }
                    // Receive
                    event = swarm.select_next_some() => match event {
                        SwarmEvent::Behaviour(BlendFarmBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                            // it would be nice to show the list of user to the UI?
                            for (peer_id, .. ) in list {
                                println!("mDNS discovered a new peer: {}", &peer_id);
                            }
                        }
                        SwarmEvent::Behaviour(BlendFarmBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                            propagation_source: peer_id,
                            message_id: id,
                            message,
                        })) => {
                            if let Ok(msg) = NetMessage::de(&message.data) {
                                println!("Got message: '{msg:?}' with id: {id} from peer: {peer_id}");
                            }
                        }
                        _ => {}
                    }
                }
                // std::thread::sleep(Duration::from_millis(100));
                // if let Ok(signal) = rx.try_recv() {
                //     let topic = gossipsub::IdentTopic::new("blendfarm-rpc-msg");
                //     let data = signal.ser();
                //     if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, data) {
                //         println!("Fail to publish message to swarm! {e:?}");
                //     }
                // }

                // // I'm worry about this one...
                // match swarm.select_next_some().await {
                //     SwarmEvent::Behaviour(BlendFarmBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                //         // it would be nice to show the list of user to the UI?
                //         for (peer_id, .. ) in list {
                //             println!("mDNS discovered a new peer: {}", &peer_id);
                //         }
                //     }
                //     SwarmEvent::Behaviour(BlendFarmBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                //         propagation_source: peer_id,
                //         message_id: id,
                //         message,
                //     })) => {
                //         if let Ok(msg) = bincode::deserialize::<NetMessage>(&message.data) {
                //             println!("Got message: '{msg:?}' with id: {id} from peer: {peer_id}");
                //         }
                //     }
                // _ => {}
                // }
            }
        });

        Ok(Self { tx, _task })
    }

    pub async fn send(
        &mut self,
        net_message: NetMessage,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.tx.send(net_message).await?;
        Ok(())
    }
}

impl AsRef<JoinHandle<()>> for NetworkService {
    fn as_ref(&self) -> &JoinHandle<()> {
        &self._task
    }
}
