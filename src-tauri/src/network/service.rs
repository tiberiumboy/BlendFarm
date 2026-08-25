use crate::constant::NODE_TOPIC;
use crate::network::behaviour::Behaviour;
use crate::network::{
    behaviour::BehaviourEvent,
    file_request::FileRequest, 
    file_response::FileResponse
};
use crate::services::file_service::{FileData, FileResult};
use crate::services::server::ServerEvent;
use crate::{
    models::behaviour::BlendFarmBehaviour,
    network::event::Event,
    network::message::Command
};
use futures::SinkExt;
use futures::StreamExt;
use futures::channel::oneshot;
use futures::channel::mpsc::{Receiver, Sender};
use libp2p::core::ConnectedPoint;
use libp2p::gossipsub::{self, IdentTopic};
use libp2p::mdns;
use libp2p::swarm::SwarmEvent;
use libp2p::{
    PeerId, Swarm,
    kad::{self, QueryId},
};
use libp2p_request_response::OutboundRequestId;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::PathBuf;
use tokio::select;

// Network service module to handle invocation commands to send to network service,
// as well as handling network event from other peers
pub struct Service {
    // swarm behaviour - interface to the network
    swarm: Swarm<Behaviour>, // TODO: Replace this back to BlendFarmBehaviour once we get network stack working again 

    // peers: HashSet<PeerId>,

    // receive Network command
    command_receiver: Receiver<Command>,

    // Send Network event to subscribers.
    event_sender: Sender<Event>,

    pending_start_providing: HashMap<QueryId, oneshot::Sender<()>>,
    pending_dial: HashMap<PeerId, oneshot::Sender<Result<(), Box<dyn Error + Send>>>>,
    providing_files: HashMap<String, PathBuf>,
    pending_get_providers: HashMap<kad::QueryId, oneshot::Sender<HashSet<PeerId>>>,
    pending_request_file:
        HashMap<OutboundRequestId, oneshot::Sender<FileResult<FileData>>>,
}

// network service will be used to handle and receive network signal. It will also transmit network package over lan
impl Service {
    pub fn new(
        swarm: Swarm<Behaviour>,
        receiver: Receiver<Command>,
        sender: Sender<Event>,
    ) -> Self {
        Self {
            swarm,
            // peers: Default::default(),
            command_receiver: receiver,
            event_sender: sender,
            pending_start_providing: Default::default(),
            pending_dial: Default::default(),
            providing_files: Default::default(),
            pending_get_providers: Default::default(),
            pending_request_file: Default::default(),
        }
    }

    /*
       From my understanding about this method implementation: broadcast all potential files and sponsor what's available.
       This methodology will change: The host will ask the client for task information that matches Job ID.
       This client will reply back to the host with list of matching task(s) information.

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
    

    // TODO: Will need to return Result<MessageId, PublishError>... For now let's keep it as-is.
    /* 
    async fn send_job_status(&mut self, event: &JobEvent) {
        let data = serde_json::to_string(&event).unwrap();
        let topic = IdentTopic::new(JOB_TOPIC);
        // we should wait until we successfully subscribed to the various topics filter.
        // The only reason why I'm getting failed to send job message is because we are not subscribed to the topic yet.
        match self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
            // TODO: Print log verbosity
            Ok(_) => println!("Job Status Sent!\n{event:?}"),
            Err(e) => eprintln!("Fail to send job message! {e:?}"),
        };
    }
    */

    // send command
    // Receive commands from foreign invocation.
    async fn handle_command(&mut self, cmd: Command) {
        // handle the commands via the services implementation given limited power for the network services.
        match cmd {
            // I'm not sure why I need this?
            /* 
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

            Command::StopListening => {
                // :think: Need more information before implementing behaviour. Do we want to stop one listener or all listeners?
                // TODO: Read note above, need to refactor this to make sense it's implementation described.
                todo!("Tell swarm to stop listening. stop all listener once lint is working again.");
            },
            */

            Command::FileService(/*service*/ .. ) => todo!("check this out later"), //self.process_file_service(service).await,

            // received server status. Can invoke commands from this broadcast event.
            Command::Message(Some(multiaddr), status) =>  {
                

                // if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                //     eprintln!("Fail to publish gossip message: {e:?}");
                // }
                
                // the method goes is that we need the self.swarm to implement the behaviour of communicating 
                // if let Err(e) = self.swarm.dial(peer_addr ) {
                //     eprintln!("Unable to dial! {e:?}");
                // }

                println!("Impl. behaviour to send a message to [{multiaddr}] {status:?}");
            }
            // Received broadcast signal
            Command::Message(_, status) => {
                // we want to send this info across broadcast network. We do not care who is listening the network. Only the fact that we want our hosts to keep notify for availability.
                let _data = serde_json::to_string(&status).unwrap();
                let _topic = IdentTopic::new(NODE_TOPIC);
                // so a relay server would be utilized here? we communicate to the peer by their id?
                todo!("Work on network stack first before implementing gossipsub");
                // if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, data) {
                //     // Can get "NoPeerSubscribedToTopic" - Should handle gracefully.
                //     eprintln!("Fail to publish gossip message: {e:?}");
                // }
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
                    self.event_sender
                        .send(Event::InboundRequest {
                            request: request.into(),
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
                        .send(Ok(response.into()));
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
                    // find a way to reveal this information via verbose logs
                    // println!("Discovered [{peer_id:?}] {address:?}");
                    
                    // create a discovery notification to the subscribers
                    // let event = Event::Discovered(peer_id, address.clone());
                    
                    // if this errors out, we should gracefully hang up?
                    // if let Err(e) = self.event_sender.send(event).await {
                    //     eprintln!("sender should not drop! {e:?}");
                    // }

                    // if I have already discovered this address, then I need to skip it. Otherwise I will produce garbage log input for duplicated peer id already exist.
                    // it seems that I do need to explicitly add the peers to the list.
                    // self.swarm
                    //     .behaviour_mut()
                    //     .gossipsub
                    //     .add_explicit_peer(&peer_id);

                    // // add the discover node to kademlia list.
                    // why would I want to do this?
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, address.clone());
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
            gossipsub::Event::Message { message, .. } => {
                // Node based event awareness
                match serde_json::from_slice::<ServerEvent>(&message.data) {
                    Ok(node_event) => {
                        println!("Received gossip message for server event: {:?}", &node_event);
                        // if let Err(e) = self.event_sender.send(Event::ServerStatus(node_event)).await {
                        //     eprintln!("Something failed? {e:?}");
                        // }
                    }
                    Err(e) => eprintln!("fail to parse Node topic data! {e:?}"),
                }
            },
            // TODO: Don't think I need this yet? suppressing this for now
            gossipsub::Event::Subscribed { peer_id, .. } => {
                // topic hash does not implement the behavior for serializer.deserialize implementations.
                // Instead assume there's only one topic to subscribed and it's global.
                let _message = ServerEvent::Joined(peer_id.to_base58());
                // if let Err(e) = self.event_sender.send(Event::ServerStatus(message)).await {
                //     eprintln!("Fail to send subscribed notification! {e:?}");
                // }
            }
            // I should be logging info from other event from gossip... wonder what they got to say?
            // TODO: Implement verbosity logging information here.
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

    // should I create a separate netowrk service for this?
    async fn handle_swarm_event(&mut self, event: SwarmEvent<BehaviourEvent>) {
        match event {
            SwarmEvent::Behaviour(behaviour) => match behaviour {
                // RequestResponse?
            // BlendFarm
            BehaviourEvent::RequestResponse(event) => {
                    self.process_response_event(event).await;
                }
                // Gossipsub used to spread message across
                // BehaviourEvent::Gossipsub(event) => {
                //     self.process_gossip_event(event).await;
                // }
                // // mdns used to identify other computer on the network
                // BehaviourEvent::Mdns(event) => {
                //     self.process_mdns_event(event).await;
                // }
                // Kademlia for DHT services
                BehaviourEvent::Kademlia(event) => {
                    self.process_kademlia_event(event).await;
                }
            },
            // Network swarm connects to you.
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                // TODO: Could we stream io?
                // TODO: Toggle verbosity mode?
                println!("Connection Established: Peer: \"{peer_id:?}\" | Endpoint: \"{endpoint:?}\"");
                match endpoint {
                    ConnectedPoint::Dialer { .. } => {
                        let Some(sender) = self.pending_dial.remove(&peer_id) else {
                            eprintln!("Unable to find matching peer id from known pending dial [{peer_id}]!");
                            return;
                        };
                        if let Err(e) = sender.send(Ok(())) {
                            eprintln!("Unable to respond back, ignoring! {e:?}");
                        }
                        println!("Dialer [{peer_id}] connection has been established!");
                    },
                    ConnectedPoint::Listener { .. } => {

                    }
                };
            }
            // why does it report I/O error? What does it mean closed by peer?
            // This was called when client starts while manager is running. "Connection error: I/O error: closed by peer: 0"
            // Lost connection to peer_id
            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                let reason = cause.and_then(|f| Some(f.to_string()));

                // Are we using ServerEvent correctly?
                let _node = ServerEvent::Disconnected {
                    peer_id: peer_id.to_base58(),
                    reason,
                };
                // let event = Event::ServerStatus(node);
                // if let Err(e) = self.event_sender.try_send(event) {
                //     eprintln!("Fail to send event on connection closed! {e:?}");
                // }
            }
            SwarmEvent::OutgoingConnectionError {
                peer_id: Some(peer_id),
                error,
                ..
            } => {
                if let Some(sender) = self.pending_dial.remove(&peer_id) {
                    if let Err(e) = sender.send(Err(Box::new(error))) {
                        eprintln!("Unable to send message to pending dialer awaiter! {e:?}");
                    }
                }
            }
            // TODO: Figure out what these events are, and see if they're any use for us to play with or delete them. Unnecessary comment codeblocks
            // SwarmEvent::ListenerClosed { .. } => todo!(),
            // SwarmEvent::ListenerError { listener_id, error } => todo!(),

            // FEATURE: Display verbose info using argument switch
            /* #region vv verbose events vv */
            SwarmEvent::OutgoingConnectionError { peer_id: None, .. } => {}
            
            // Suppressing logs
            SwarmEvent::Dialing { .. } => {}
            SwarmEvent::IncomingConnection {..} => {} 
            SwarmEvent::NewListenAddr { /* address, */ .. } => {
                // println!("[New Listener Address]: {address}");
                // let local_peer_id = *self.swarm.local_peer_id();
                // TODO: Find a way to make this as verbose option
                // eprintln!(
                //     "Listening @ {:?}",
                //     address
                //     // address.with(Protocol::P2p(local_peer_id))
                // );
            }
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

        // right now I need to find a way to spin up a loop to handle internal process.
        loop {
            select! {
                // Receive network event
                event = self.swarm.select_next_some() => self.handle_swarm_event(event).await,
                // handle command received
                Ok(command) = self.command_receiver.recv() => self.handle_command(command).await,
            }
        }
    }
}


#[cfg(test)]
pub mod test {
    // TODO: perform some service test. How can I get the service up and running for this?

    // successful test
    pub(crate) fn mock_service() {

    }
}
