use crate::constant::{JOB_TOPIC, NODE_TOPIC};
use crate::models::behaviour::{BlendFarmBehaviourEvent, FileRequest, FileResponse};
use crate::models::job::JobEvent;
use crate::network::message::{ChannelStatus, FileCommand, NodeEvent};
use crate::{
    models::behaviour::BlendFarmBehaviour,
    network::message::{Command, Event},
};
use futures::StreamExt;
use futures::channel::oneshot;
use libp2p::gossipsub::{self, IdentTopic};
use libp2p::kad::RecordKey;
use libp2p::mdns;
use libp2p::multiaddr::Protocol;
use libp2p::swarm::SwarmEvent;
use libp2p::{
    Multiaddr, PeerId, Swarm,
    kad::{self, QueryId},
};
use libp2p_request_response::OutboundRequestId;
use std::collections::{HashMap, HashSet, hash_map};
use std::error::Error;
use std::path::PathBuf;
use tokio::select;
use tokio::sync::mpsc::{Receiver, Sender};

// Network service module to handle invocation commands to send to network service,
// as well as handling network event from other peers
pub struct Service {
    // swarm behaviour - interface to the network
    swarm: Swarm<BlendFarmBehaviour>,

    // receive Network command
    receiver: Receiver<Command>,

    // Send Network event to subscribers.
    sender: Sender<Event>,

    // connection established
    dialers: HashMap<PeerId, Multiaddr>,

    pending_start_providing: HashMap<QueryId, oneshot::Sender<()>>,
    pending_dial: HashMap<PeerId, oneshot::Sender<Result<(), Box<dyn Error + Send>>>>,
    providing_files: HashMap<String, PathBuf>,
    pending_get_providers: HashMap<kad::QueryId, oneshot::Sender<HashSet<PeerId>>>,
    pending_request_file:
        HashMap<OutboundRequestId, oneshot::Sender<Result<Vec<u8>, Box<dyn Error + Send>>>>,
}

// network service will be used to handle and receive network signal. It will also transmit network package over lan
impl Service {
    pub fn new(
        swarm: Swarm<BlendFarmBehaviour>,
        receiver: Receiver<Command>,
        sender: Sender<Event>,
    ) -> Self {
        Self {
            swarm,
            receiver,
            sender,
            dialers: Default::default(),
            pending_start_providing: Default::default(),
            pending_dial: Default::default(),
            providing_files: Default::default(),
            pending_get_providers: Default::default(),
            pending_request_file: Default::default(),
        }
    }

    /*
       From my understanding about this method implementation is that we wanted to be able to broadcast
       all of the potential files out there and sponsor what's available.
       I think this methodology will change because we wanted the host to ask the client if there's any files available
       or completed by this machine, and then reply back to the host.

       I need to setup a network diagram to make this network layer protocol clear and understand,
       as well as easy to debug, test, and identify potential issues.

       From the host side. the host will broadcast asking for job updates.
       This update will include job id.

       On the client side, the client will receive the notification from the host,
       and check the database to see if the job id exist.

       if it does exist, then the client will broadcast list of completed images.
       The host will receive this list and compare to the host machine to see if they have the image

       If the host does not have the image, it will initiate a file transfer between the host and the client machine
       In this case, we should not have to make all of the files available, but instead make the target image
       available for the host to transfer over the network protocol.

       This is recognized as a tcp handshake connection, asking for the image from the node
       and the node will send the image via channel request.
    */

    // here we will deviate handling the file service command.
    async fn process_file_service(&mut self, cmd: FileCommand) {
        match cmd {
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
            FileCommand::StartProviding(keyword, file_path) => {
                let key = keyword.clone().into_bytes().into();
                // could we make use of this query ID?
                let _query_id = self
                    .swarm
                    .behaviour_mut()
                    .kademlia
                    .start_providing(key)
                    .expect("No store error.");
                self.providing_files.insert(keyword, file_path);
            }
            FileCommand::StopProviding(keyword) => {
                let key = RecordKey::new(&keyword.as_bytes());
                self.swarm.behaviour_mut().kademlia.stop_providing(&key);
                self.providing_files.remove(&keyword);
            }
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

    // send command
    // Receive commands from foreign invocation.
    async fn handle_command(&mut self, cmd: Command) {
        // handle the commands via the services implementation given limited power for the network services.
        match cmd {
            Command::Subscribe { topic } => {
                let identity = IdentTopic::new(topic);
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.subscribe(&identity) {
                    eprintln!("Fail to subscribe! {e:}");
                }
            }
            Command::StartListening { addr, sender } => {
                let _result = match self.swarm.listen_on(addr) {
                    Err(e) => match e.source() {
                        Some(err) => Err(Box::new(err.to_string())),
                        None => Ok(()),
                    },
                    _ => Ok(()),
                };
                // TODO, figure out how to get this situation straighten? Why
                // sender.send(result);
                if let Err(e) = sender.send(Ok(())) {
                    eprintln!("Fail to send! {e:?}");
                }
            }

            Command::Dial {
                peer_id,
                peer_addr,
                sender,
            } => {
                if let hash_map::Entry::Vacant(e) = self.pending_dial.entry(peer_id) {
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, peer_addr.clone());

                    match self.swarm.dial(peer_addr.with(Protocol::P2p(peer_id))) {
                        Ok(()) => {
                            e.insert(sender);
                        }
                        Err(e) => {
                            sender.send(Err(Box::new(e))).expect("Should not drop");
                        }
                    }
                } else {
                    eprintln!("Already dialing the peer!");
                }
            }

            // use this to advertise files. On app startup we should broadcast blender apps as well.
            Command::StartProviding { file_name, sender } => {
                // TODO: Find a way to get around expect()!
                let query_id = self
                    .swarm
                    .behaviour_mut()
                    .kademlia
                    .start_providing(file_name.into_bytes().into())
                    .expect("No store value");
                self.pending_start_providing.insert(query_id, sender);
            }

            Command::GetProviders { file_name, sender } => {
                let query_id = self
                    .swarm
                    .behaviour_mut()
                    .kademlia
                    .get_providers(file_name.into_bytes().into());
                self.pending_get_providers.insert(query_id, sender);
            }
            Command::RequestFile {
                file_name,
                peer,
                sender,
            } => {
                let request_id = self
                    .swarm
                    .behaviour_mut()
                    .request_response
                    .send_request(&peer, FileRequest(file_name));
                self.pending_request_file.insert(request_id, sender);
            }
            Command::RespondFile { file, channel } => {
                self.swarm
                    .behaviour_mut()
                    .request_response
                    .send_response(channel, FileResponse(file))
                    .expect("Connection to peer should be still open");
            }
            Command::FileService(service) => self.process_file_service(service).await,

            // received job status. invoke commands
            // we should only send command if we are subscribed.
            Command::JobStatus(event) => {
                // I want to send a message only if we have active subscribers.
                // which means I need to create my own list of peers I think may be listening on the network
                // convert data into json format.
                // The foreign request is asking for the Job Status -> Reply back to the user directly.
                let data = serde_json::to_string(&event).unwrap();
                let topic = IdentTopic::new(JOB_TOPIC);
                // we should wait until we successfully subscribed to the various topics filter.
                // The only reason why I'm getting failed to send job message is because we are not subscribed to the topic yet.
                // how can I wait until we're subscribed to the topic?
                match self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                    // TODO: Print log verbosity
                    Ok(_) => println!("Job Status Sent!\n{event:?}"),
                    Err(e) => eprintln!("Fail to send job message! {e:?}"),
                };
            }
            Command::NodeStatus(status) => {
                // we want to send this info across broadcast network. We do not care who is listening the network. Only the fact that we want our hosts to keep notify for availability.
                let data = serde_json::to_string(&status).unwrap();
                let topic = IdentTopic::new(NODE_TOPIC);
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                    eprintln!("Fail to publish gossip message: {e:?}");
                }
            }
        }
    }

    // This method is invoked by network event.
    // This is under RequestResponse
    async fn process_response_event(
        &mut self,
        event: libp2p_request_response::Event<FileRequest, FileResponse>,
    ) {
        match event {
            libp2p_request_response::Event::Message { message, .. } => match message {
                libp2p_request_response::Message::Request {
                    request, channel, ..
                } => {
                    self.sender
                        .send(Event::InboundRequest {
                            request: request.0,
                            channel,
                        })
                        .await
                        .expect("Event receiver should not be dropped!");
                }
                libp2p_request_response::Message::Response {
                    request_id,
                    response,
                } => {
                    let _ = self
                        .pending_request_file
                        .remove(&request_id)
                        .expect("Request to still be pending")
                        .send(Ok(response.0));
                }
            },
            libp2p_request_response::Event::OutboundFailure {
                request_id, error, ..
            } => {
                let _ = self
                    .pending_request_file
                    .remove(&request_id)
                    .expect("Request to still be pending")
                    .send(Err(Box::new(error)));
            }
            libp2p_request_response::Event::ResponseSent { .. } => {}
            _ => {}
        }
    }

    async fn process_mdns_event(&mut self, event: mdns::Event) {
        match event {
            mdns::Event::Discovered(peers) => {
                for (peer_id, address) in peers {
                    println!("Discovered [{peer_id:?}] {address:?}");

                    // when I process this, how do I know where dialers is used?
                    let event = Event::Discovered(peer_id, address);
                    if let Err(e) = self.sender.send(event).await {
                        eprintln!("sender should not drop! {e:?}");
                    }

                    // if I have already discovered this address, then I need to skip it. Otherwise I will produce garbage log input for duplicated peer id already exist.
                    // it seems that I do need to explicitly add the peers to the list.
                    // self.swarm
                    //     .behaviour_mut()
                    //     .gossipsub
                    //     .add_explicit_peer(&peer_id);

                    // // add the discover node to kademlia list.
                    // why would I want to do this?
                    // self.swarm
                    //     .behaviour_mut()
                    //     .kad
                    //     .add_address(&peer_id, address.clone());
                }
            }
            mdns::Event::Expired(..) => {
                // for (peer_id, ..) in peers {
                //     self.swarm
                //         .behaviour_mut()
                //         .gossipsub
                //         .remove_explicit_peer(&peer_id);
                // }
            }
        };
    }

    async fn process_gossip_event(&mut self, event: gossipsub::Event) {
        match event {
            // what is propagation source? can we use this somehow?
            gossipsub::Event::Message { message, .. } => match message.topic.as_str() {
                // if the topic is JOB related, assume data as JobEvent
                JOB_TOPIC => match serde_json::from_slice::<JobEvent>(&message.data) {
                    Ok(job_event) => {
                        if let Err(e) = self.sender.send(Event::JobUpdate(job_event)).await {
                            eprintln!("Something failed? {e:?}");
                        }
                    }
                    Err(e) => {
                        eprintln!("Fail to parse Job topic data! {e:?}");
                    }
                },
                // Node based event awareness
                NODE_TOPIC => match serde_json::from_slice::<NodeEvent>(&message.data) {
                    Ok(node_event) => {
                        if let Err(e) = self.sender.send(Event::NodeStatus(node_event)).await {
                            eprintln!("Something failed? {e:?}");
                        }
                    }
                    Err(e) => eprintln!("fail to parse Node topic data! {e:?}"),
                },

                // Garbage collector - Treat this as a grain of salt. Do not execute any data from this scope
                // should only be used to display logs and info, things for us to identify unusual activity going on outside our domain specification.
                _ => {
                    // I received Mac.lan from message.topic?
                    let topic = message.topic.as_str();
                    eprintln!("Intercepted unhandled signal here: {topic}");
                }
            },
            gossipsub::Event::Subscribed { peer_id, topic } => {
                // what are the peer_id and topic?
                // Maybe it's the user who joined the network, we can send a RequestTask if we're idle?
                let update = ChannelStatus::Joined( peer_id, topic );
                let event = Event::Channel(update);
                self.sender.send(event).await;
            }
            // I should be logging info from other event from gossip... wonder what they got to say?
            // TODO: Log and verify if we need to handle other gossip events.
            any => {
                println!("[Unhandled Gossipsub]{any:?}");
            }
        }
    }

    // async fn process_outbound_query(&mut )

    // Handle kademila events (Used for file sharing)
    // can we use this same DHT to make node spec publicly available?
    async fn process_kademlia_event(&mut self, kad_event: kad::Event) {
        match kad_event {
            kad::Event::OutboundQueryProgressed {
                id: query_id,
                result: query_result,
                ..
            } => {
                match query_result {
                    kad::QueryResult::StartProviding(..) => {
                        let sender: oneshot::Sender<()> = self
                            .pending_start_providing
                            .remove(&query_id)
                            .expect("Completed query to be previously pending.");
                        let _ = sender.send(());
                    }
                    kad::QueryResult::GetProviders(Ok(kad::GetProvidersOk::FoundProviders {
                        providers,
                        ..
                    })) => {
                        if let Some(sender) = self.pending_get_providers.remove(&query_id) {
                            sender.send(providers).expect("Receiver not to be dropped");

                            if let Some(mut node) =
                                self.swarm.behaviour_mut().kademlia.query_mut(&query_id)
                            {
                                node.finish();
                            }
                        }
                    }
                    kad::QueryResult::GetProviders(Ok(
                        kad::GetProvidersOk::FinishedWithNoAdditionalRecord { .. },
                    )) => {
                        // yeah this looks wrong?
                        if let Some(sender) = self.pending_get_providers.remove(&query_id) {
                            sender
                                .send(HashSet::new())
                                .expect("Sender not to be dropped");
                        }

                        if let Some(mut node) =
                            self.swarm.behaviour_mut().kademlia.query_mut(&query_id)
                        {
                            node.finish();
                        }
                        // This piece of code means that there's nobody advertising this on the network?
                        // what was suppose to happen here?
                        // TODO: I am once again stopped here. This message appeared from the CLI side. Not the host.

                        // let outbound_request_id = id;
                        // let event = Event::PendingRequestFiled(outbound_request_id, None);
                        // self.sender.send(event).await;
                    }
                    kad::QueryResult::PutRecord(result) => match result {
                        Ok(value) => println!("Successfully append the record! {value:?}"),
                        Err(e) => eprintln!("Error putting record in! {e:?}"),
                    },
                    // suppressed
                    _ => {}
                }
            }

            kad::Event::InboundRequest { .. } => {} // suppressed
            kad::Event::RoutingUpdated { .. } => {} // suppressed
            // TODO: Find out what cause this to happen and see if we need to handle anything for this invocation exception
            kad::Event::UnroutablePeer { peer } => {
                eprintln!("Unroutable Peer? {peer}");
            } // suppressed
            _ => {
                // oh mah gawd. What am I'm suppose to do here?
                eprintln!("Unhandled Kademila event: {kad_event:?}");
            }
        }
    }

    // Process incoming network events - Treat this as receiving new orders.
    async fn handle_event(&mut self, event: SwarmEvent<BlendFarmBehaviourEvent>) {
        match event {
            SwarmEvent::Behaviour(behaviour) => match behaviour {
                // RequestResponse?
                BlendFarmBehaviourEvent::RequestResponse(event) => {
                    self.process_response_event(event).await;
                }
                // Gossipsub used to spread message across
                BlendFarmBehaviourEvent::Gossipsub(event) => {
                    self.process_gossip_event(event).await;
                }
                // mdns used to identify other computer on the network
                BlendFarmBehaviourEvent::Mdns(event) => {
                    self.process_mdns_event(event).await;
                }
                // Kademlia for DHT services
                BlendFarmBehaviourEvent::Kademlia(event) => {
                    self.process_kademlia_event(event).await;
                }
            },
            // So how does the established works?
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                println!("Connection Established: {peer_id:?}\n{endpoint:?}");

                if endpoint.is_dialer() {
                    if let Some(sender) = self.pending_dial.remove(&peer_id) {
                        self.dialers
                            .entry(peer_id)
                            .and_modify(|f| *f = endpoint.get_remote_address().clone());
                        let _ = sender.send(Ok(()));
                    }
                }
            }
            // This was called when client starts while manager is running. "Connection error: I/O error: closed by peer: 0"
            // TODO: Read what ConnectionClosed does?
            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                let reason = cause.and_then(|f| Some(f.to_string()));
                let node = NodeEvent::Disconnected {
                    peer_id: peer_id.to_base58(),
                    reason,
                };
                let event = Event::NodeStatus(node);
                if let Err(e) = self.sender.send(event).await {
                    eprintln!("Fail to send event on connection closed! {e:?}");
                }
            }
            SwarmEvent::OutgoingConnectionError {
                peer_id: Some(peer_id),
                error,
                ..
            } => {
                if let Some(sender) = self.pending_dial.remove(&peer_id) {
                    let _ = sender.send(Err(Box::new(error)));
                }
            }
            // TODO: Figure out what these events are, and see if they're any use for us to play with or delete them. Unnecessary comment codeblocks
            // SwarmEvent::ListenerClosed { .. } => todo!(),
            // SwarmEvent::ListenerError { listener_id, error } => todo!(),

            // FEATURE: Display verbose info using argument switch
            /* #region vv verbose events vv */
            SwarmEvent::OutgoingConnectionError { peer_id: None, .. } => {}

            SwarmEvent::NewListenAddr { address, .. } => {
                // println!("[New Listener Address]: {address}");
                let local_peer_id = *self.swarm.local_peer_id();
                eprintln!(
                    "Listening @ {:?}",
                    address.with(Protocol::P2p(local_peer_id))
                );
            }

            SwarmEvent::Dialing { .. } => {}

            SwarmEvent::IncomingConnection {
                connection_id,
                local_addr,
                send_back_addr,
            } => {
                // Incoming connection? How do I accept?
                eprintln!("Incoming connection: {connection_id} | {local_addr} | {send_back_addr}");

                // I'm assuming this is reply from dial?
                // what does it mean to have incoming connection here?
                // self.dialers.entry()
            } // Suppressing logs

            // Suppressing logs
            SwarmEvent::NewExternalAddrOfPeer { .. } => {}
            SwarmEvent::IncomingConnectionError { .. } => {} // I recognize this and do want to display result below.
            SwarmEvent::ExpiredListenAddr { .. } => {}

            /* #endregion ^^eof ignore^^ */
            // Must fully exhaust all condition types as possible!
            // Add to the ignore list with description why we're suppressing logs. They must be visible under verbose mode.
            e => panic!("{e:?}"),
        };
    }

    // run the network loops
    pub(crate) async fn run(mut self) {
        loop {
            select! {
                event = self.swarm.select_next_some() => self.handle_event(event).await,
                command = self.receiver.recv() => match command {
                    Some(c) => self.handle_command(c).await,
                    None => return,
                },
            }
        }
    }
}

#[cfg(test)]
pub mod test {
    // TODO: perform some service test. How can I get the service up and running for this?
}
