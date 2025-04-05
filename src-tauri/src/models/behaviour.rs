use futures::channel::oneshot;
use libp2p::{
    gossipsub::{self, IdentTopic},
    kad::{self, RecordKey},
    mdns, ping,
    swarm::{NetworkBehaviour, SwarmEvent},
    PeerId,
};
use libp2p_request_response::{cbor, OutboundRequestId};
use machine_info::Machine;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
};
use tokio::sync::mpsc::Sender;

use crate::models::job::JobEvent;

use super::{
    computer_spec::ComputerSpec,
    message::{NetCommand, NetEvent},
    network::{SPEC, STATUS},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRequest(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileResponse(pub Vec<u8>);

pub struct FileService {
    pub pending_get_providers: HashMap<kad::QueryId, oneshot::Sender<HashSet<PeerId>>>,
    pub pending_start_providing: HashMap<kad::QueryId, oneshot::Sender<()>>,
    pub pending_request_file:
        HashMap<OutboundRequestId, oneshot::Sender<Result<Vec<u8>, Box<dyn Error + Send>>>>,
}

impl FileService {
    pub fn new() -> Self {
        FileService {
            pending_get_providers: HashMap::new(),
            pending_start_providing: HashMap::new(),
            pending_request_file: HashMap::new(),
        }
    }
}

#[derive(NetworkBehaviour)]
pub struct BlendFarmBehaviour {
    pub ping: ping::Behaviour,
    // file transfer response protocol
    pub request_response: cbor::Behaviour<FileRequest, FileResponse>,
    // Communication between peers to pepers
    pub gossipsub: gossipsub::Behaviour,
    // self discovery network service
    pub mdns: mdns::tokio::Behaviour,
    // used to provide file availability
    pub kad: kad::Behaviour<kad::store::MemoryStore>,
}

// would this work for me?
impl BlendFarmBehaviour {
    // send command
    // is it possible to not use self?
    pub async fn handle_command(&mut self, file_service: &mut FileService, cmd: NetCommand) {
        match cmd {
            NetCommand::Status(msg) => {
                let data = msg.as_bytes();
                let topic = IdentTopic::new(STATUS);
                if let Err(e) = self.gossipsub.publish(topic, data) {
                    eprintln!("Fail to send status over network! {e:?}");
                }
            }
            NetCommand::RequestFile {
                peer_id,
                file_name,
                sender,
            } => {
                let request_id = self
                    .request_response
                    .send_request(&peer_id, FileRequest(file_name.into()));

                file_service.pending_request_file.insert(request_id, sender);
            }
            NetCommand::RespondFile { file, channel } => {
                // somehow the send_response errored out? How come?
                // Seems like this function got timed out?
                if let Err(e) = self
                    .request_response
                    // TODO: find a way to get around cloning values.
                    .send_response(channel, FileResponse(file.clone()))
                {
                    // why am I'm getting error message here?
                    eprintln!("Error received on sending response!");
                }
            }
            NetCommand::IncomingWorker(..) => {
                let mut machine = Machine::new();
                let spec = ComputerSpec::new(&mut machine);
                let data = bincode::serialize(&spec).unwrap();
                let topic = IdentTopic::new(SPEC);
                // let _ = swarm.dial(peer_id);    // so close... yet why?
                if let Err(e) = self.gossipsub.publish(topic, data) {
                    eprintln!("Fail to send identity to swarm! {e:?}");
                };
            }
            NetCommand::GetProviders { file_name, sender } => {
                let key = RecordKey::new(&file_name.as_bytes());
                let query_id = self.kad.get_providers(key.into());
                file_service.pending_get_providers.insert(query_id, sender);
            }
            NetCommand::StartProviding { file_name, sender } => {
                let provider_key = RecordKey::new(&file_name.as_bytes());
                let query_id = self
                    .kad
                    .start_providing(provider_key)
                    .expect("No store error.");

                file_service
                    .pending_start_providing
                    .insert(query_id, sender);
            }
            NetCommand::SubscribeTopic(topic) => {
                let ident_topic = IdentTopic::new(topic);
                self.gossipsub.subscribe(&ident_topic).unwrap();
            }
            NetCommand::UnsubscribeTopic(topic) => {
                let ident_topic = IdentTopic::new(topic);
                self.gossipsub.unsubscribe(&ident_topic);
            }
            // for the time being we'll use gossip.
            // TODO: For future impl. I would like to target peer by peer_id instead of host name.
            NetCommand::JobStatus(host_name, event) => {
                // convert data into json format.
                let data = bincode::serialize(&event).unwrap();

                // currently using a hack by making the target machine subscribe to their hostname.
                // the manager will send message to that specific hostname as target instead.
                // TODO: Read more about libp2p and how I can just connect to one machine and send that machine job status information.
                let topic = IdentTopic::new(host_name);
                if let Err(e) = self.gossipsub.publish(topic, data) {
                    eprintln!("Error sending job status! {e:?}");
                }

                /*
                Let's break this down, we receive a worker with peer_id and peer_addr, both of which will be used to establish communication
                Once we establish a communication, that target peer will need to receive the pending task we have assigned for them.
                For now, we will try to dial the target peer, and append the task to our network service pool of pending task.
                */
                // self.pending_task.insert(peer_id);
            }
            NetCommand::Dial {
                peer_id,
                peer_addr,
                sender,
            } => {
                println!(
                    "Dialed: \nid:{:?}\naddr:{:?}\nsender:{:?}",
                    peer_id, peer_addr, sender
                );
                // Ok so where is this coming from?
                // if let hash_map::Entry::Vacant(e) = self.pending_dial.entry(peer_id) {
                //     behaviour
                //     .kad
                //     .add_address(&peer_id, peer_addr.clone());

                // match swarm.dial(peer_addr.with(Protocol::P2p(peer_id))) {
                //     Ok(()) => {
                //         e.insert(sender);
                //     }
                //     Err(e) => {
                //         let _ = sender.send(Err(Box::new(e)));
                //     }
                // }
            }
        }
    }

    pub async fn handle_event(
        &mut self,
        sender: &mut Sender<NetEvent>,
        file_service: &mut FileService,
        event: &SwarmEvent<BlendFarmBehaviourEvent>,
    ) {
        match event {
            SwarmEvent::Behaviour(BlendFarmBehaviourEvent::Mdns(mdns)) => {
                self.handle_mdns(&mdns).await
            }
            SwarmEvent::Behaviour(BlendFarmBehaviourEvent::Gossipsub(gossip)) => {
                Self::handle_gossip(sender, &gossip).await;
            }
            SwarmEvent::Behaviour(BlendFarmBehaviourEvent::Kad(kad)) => {
                self.handle_kademila(&mut file_service, &kad).await
            }
            SwarmEvent::Behaviour(BlendFarmBehaviourEvent::RequestResponse(rr)) => {
                Self::handle_response(sender, &mut file_service, rr).await
            }
            // Once the swarm establish connection, we then send the peer_id we connected to.
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                sender
                    .send(NetEvent::OnConnected(peer_id.clone()))
                    .await
                    .unwrap();
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                sender
                    .send(NetEvent::NodeDisconnected(peer_id.clone()))
                    .await
                    .unwrap();
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                // hmm.. I need to capture the address here?
                // how do I save the address?
                // this seems problematic?
                // if address.protocol_stack().any(|f| f.contains("tcp")) {
                //     self.public_addr = Some(address);
                // }
            }
            _ => {} //println!("[Network]: {event:?}");
        }
    }

    async fn handle_response(
        sender: &mut Sender<NetEvent>,
        file_service: &mut FileService,
        event: &libp2p_request_response::Event<FileRequest, FileResponse>,
    ) {
        match event {
            libp2p_request_response::Event::Message { message, .. } => match message {
                libp2p_request_response::Message::Request {
                    request, channel, ..
                } => {
                    sender
                        .send(NetEvent::InboundRequest {
                            request: request.0,
                            channel: channel.into(),
                        })
                        .await
                        .expect("Event receiver should not be dropped!");
                }
                libp2p_request_response::Message::Response {
                    request_id,
                    response,
                } => {
                    if let Err(e) = file_service
                        .pending_request_file
                        .remove(&request_id)
                        .expect("Request is still pending?")
                        .send(Ok(response.0))
                    {
                        eprintln!("libp2p Response Error: {e:?}");
                    }
                }
            },
            libp2p_request_response::Event::OutboundFailure {
                request_id, error, ..
            } => {
                if let Err(e) = file_service
                    .pending_request_file
                    .remove(&request_id)
                    .expect("Request is still pending")
                    .send(Err(Box::new(error)))
                {
                    eprintln!("libp2p outbound fail: {e:?}");
                }
            }
            libp2p_request_response::Event::ResponseSent { .. } => {}
            _ => {}
        }
    }

    async fn handle_mdns(&mut self, event: &mdns::Event) {
        match event {
            mdns::Event::Discovered(peers) => {
                for (peer_id, address) in peers {
                    self.gossipsub.add_explicit_peer(&peer_id);

                    // add the discover node to kademlia list.
                    self.kad.add_address(&peer_id, address.clone());
                }
            }
            mdns::Event::Expired(peers) => {
                for (peer_id, ..) in peers {
                    self.gossipsub.remove_explicit_peer(&peer_id);
                }
            }
        };
    }

    // TODO: Figure out how I can use the match operator for TopicHash. I'd like to use the TopicHash static variable above.
    async fn handle_gossip(sender: &mut Sender<NetEvent>, event: &gossipsub::Event) {
        match event {
            gossipsub::Event::Message { message, .. } => match message.topic.as_str() {
                SPEC => {
                    let source = message.source.expect("Source cannot be empty!");
                    let specs =
                        bincode::deserialize(&message.data).expect("Fail to parse Computer Specs!");
                    if let Err(e) = sender.send(NetEvent::NodeDiscovered(source, specs)).await {
                        eprintln!("Something failed? {e:?}");
                    }
                }
                STATUS => {
                    let source = message.source.expect("Source cannot be empty!");
                    // this looks like a bad idea... any how we could not use clone? stream?
                    let msg = String::from_utf8(message.data.clone()).unwrap();
                    if let Err(e) = sender.send(NetEvent::Status(source, msg)).await {
                        eprintln!("Something failed? {e:?}");
                    }
                }
                JOB => {
                    // let peer_id = self.swarm.local_peer_id();
                    let job_event = bincode::deserialize::<JobEvent>(&message.data)
                        .expect("Fail to parse Job data!");

                    // I don't think this function is called?
                    println!("Is this function used?");
                    if let Err(e) = sender.send(NetEvent::JobUpdate(job_event)).await {
                        eprintln!("Something failed? {e:?}");
                    }
                }
                // I think this needs to be changed.
                _ => {
                    eprintln!(
                        "Received unhandled gossip event: \n{}",
                        message.topic.as_str()
                    );
                    todo!("Find a way to return the data we received from the network node. We could instead just figure out about the machine's hostname somewhere else");

                    // let topic = message.topic.as_str();
                    // if topic.eq(&self.machine.system_info().hostname) {
                    //     let job_event = bincode::deserialize::<JobEvent>(&message.data)
                    //         .expect("Fail to parse job data!");
                    //     if let Err(e) = sender
                    //         .send(NetEvent::JobUpdate(topic.to_string(), job_event))
                    //         .await
                    //     {
                    //         eprintln!("Fail to send job update!\n{e:?}");
                    //     }
                    // } else {
                    //     // let data = String::from_utf8(message.data).unwrap();
                    //     println!("Intercepted unhandled signal here: {topic}");
                    //     // TODO: We may intercept signal for other purpose here, how can I do that?
                    // }
                }
            },
            _ => {}
        }
    }

    // Handle kademila events (Used for file sharing)
    // thinking about transferring this to behaviour class?
    async fn handle_kademila(&mut self, file_service: &mut FileService, event: &kad::Event) {
        match event {
            kad::Event::OutboundQueryProgressed {
                id,
                result: kad::QueryResult::StartProviding(_),
                ..
            } => {
                let sender: oneshot::Sender<()> = file_service
                    .pending_start_providing
                    .remove(&id)
                    .expect("Completed query to be previously pending.");
                let _ = sender.send(());
            }
            kad::Event::OutboundQueryProgressed {
                id,
                result:
                    kad::QueryResult::GetProviders(Ok(kad::GetProvidersOk::FoundProviders {
                        providers,
                        ..
                    })),
                ..
            } => {
                if let Some(sender) = file_service.pending_get_providers.remove(&id) {
                    sender
                        .send(providers.clone())
                        .expect("Receiver not to be dropped");
                    self.kad.query_mut(&id).unwrap().finish();
                }
            }
            kad::Event::OutboundQueryProgressed {
                result:
                    kad::QueryResult::GetProviders(Ok(
                        kad::GetProvidersOk::FinishedWithNoAdditionalRecord { .. },
                    )),
                ..
            } => {
                // what was suppose to happen here?
                println!(
                    r#"On OutboundQueryProgressed with result filter of 
                FinishedWithNoAdditionalRecord: This should do something?"#
                );
            }
            _ => {
                eprintln!("Unhandle Kademila event: {event:?}");
            }
        }
    }
}
