/*
Have a look into TUI for CLI status display window to show user entertainment on screen
https://docs.rs/tui/latest/tui/

Feature request:
    - See how we can treat this application process as service mode so that it can be initialize and start on machine reboot?
    - receive command to properly reboot computer when possible?
*/
use super::blend_farm::BlendFarm;
use crate::domains::render_store::RenderStore;
use crate::domains::ticket_store::{TicketError, TicketStore};
use crate::models::computer_spec::ComputerSpec;
use crate::models::job::JobId;
use crate::network::PeerIdString;
use crate::network::message::{self, Event, NetworkError};
use crate::network::provider_rule::ProviderRule;
use crate::services::app_context::AppContext;
use crate::services::blend_farm::BlendFarmError;
use crate::services::data_store::sqlite_renders_store::SqliteRenderStore;
use crate::services::data_store::sqlite_ticket_store::SqliteTicketStore;
use crate::{
    models::{job::Job, server_setting::ServerSetting, ticket::Ticket},
    network::controller::Controller as NetworkController,
};
use async_lock::RwLock;
use async_trait::async_trait;
use blender::blender::{Frame, Manager as BlenderManager, ManagerError};
use blender::models::event::BlenderEvent;
use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::async_runtime::Receiver;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use tokio::{select /* spawn */};
use uuid::Uuid;

// this is invocation commands. Signal to start, stop, fetch blender information and relative info.
#[allow(dead_code)]
enum ServerCommand {
    Start,
    AddTicket(Ticket),
    DeleteTicket(Uuid),
    CheckBlender(PeerId, String), // Name of the blender in compressed package enum. (e.g. "blender-5.0.0-linux-x64.tar.xz")
    // this function seems confusing. Refine this a bit letter.
    Fetch(JobId, oneshot::Sender<HashMap<Frame, PathBuf>>),
    Abort,
}

// Must be serializable to send data across network
// issue with this is that this cannot be convert into Encode,Decode by bincode. Instead we'll have to
#[derive(Debug, Serialize, Deserialize)]
pub enum ServerEvent {
    // Node joined the network
    Online(Multiaddr, ComputerSpec),
    // Network received a disconnected signal from peer_id.
    Disconnected {
        peer_id: PeerIdString,
        reason: Option<String>,
    },
    // I wonder if we want to send notification stating that this client join the server.
    Joined(PeerIdString), // should we care which topic this peer id joined?
    // TODO: Rendering... what?
    Rendering(Uuid),
    // Receive blender status information
    BlenderStatus(BlenderEvent),
    // Request list of completed renders by job id.
    RequestJobInfo(JobId),
    // Broadcast to remove target job id
    RemoveJob(JobId),
    // Broadcast requesting a ticket, multiaddr is the requestor
    RequestTicket(Multiaddr),
    NewTickets(Ticket),
}

#[derive(Debug, Error)]
enum ServerError {
    #[error("Encounter an network error! \n{0:}")]
    NetworkError(#[from] message::NetworkError),
    #[error("Encounter an IO error! \n{0}")]
    Io(#[from] async_std::io::Error),
    #[error("Manager Error: {0}")]
    ManagerError(#[from] ManagerError),
    #[error("Task Error: {0}")]
    TaskError(#[from] TicketError),
}

/// The behaviour described in the Cli App can be summarize below:
/// When running with listening server, client will spin a thread to listen for network messages.
///     Cli as a listening server can accept Request ticket from available host.
///     The host can ask you about your task's progress, and how many image you've completed.
///     Additionally, the host may also request the images from you.
/// When running in pure cli mode, you can ask to fetch information and create new ticket from local machine.
/// This will let cli mode run the job in batch mode, customized for your experience.
/// and simply closes out.
/// This will be useful for blender add-on interface, we want to be able to invoke client/host commands from blender application, as an alternative solution.
pub struct Server {
    manager: Arc<RwLock<BlenderManager>>,

    // database connection
    db_conn: Pool<Sqlite>,

    // config
    settings: ServerSetting,

    // current server specs
    pub spec: ComputerSpec,
}

// cli app should really be a stateless machine. A listener would just receive order from the network and proceed the ticket given queued.
// This program should close after completing the ticket queue, in non-listening mode
impl Server {
    pub(crate) fn new(context: AppContext, db: &Pool<Sqlite>) -> Self {
        Self {
            settings: context.settings,
            manager: Arc::new(RwLock::new(context.manager)),
            db_conn: db.clone(),
            spec: ComputerSpec::new(),
        }
    }

    // This function will ensure the directory will exist, and return the path to that given directory.
    // It will remain valid unless directory or parent above is removed during runtime.
    async fn generate_temp_project_task_directory(
        settings: &ServerSetting,
        task: &Ticket,
        id: &str,
    ) -> Result<PathBuf, async_std::io::Error> {
        // create a path link where we think the file should be
        let job = AsRef::<Job>::as_ref(&task);
        let project_path = settings
            .blend_dir
            .join(id.to_string())
            .join(&job.get_file_name_expected());

        // we only want the parent directory to exist.
        match async_std::fs::create_dir_all(&project_path.parent().expect("I wouldn't think we'd be trying to check files in root? Please write a bug report and replicate step by step to reproduce the issue")).await {
            Ok(_) => Ok(project_path),
            Err(e) => {
                Err(e)
            }
        }
    }

    #[allow(dead_code)]
    async fn validate_project_file(
        &self,
        client: &mut NetworkController,
        task: &Ticket,
    ) -> Result<PathBuf, ServerError> {
        let id = AsRef::<Uuid>::as_ref(&task);
        let project_file_path =
            Server::generate_temp_project_task_directory(&self.settings, &task, &id.to_string())
                .await
                .expect("Should have permission!");

        // assume project file is located inside this directory.
        println!("Checking for {:?}", &project_file_path);

        let job = AsRef::<Job>::as_ref(&task);
        // Fetch the project from peer if we don't have it.
        if !project_file_path.exists() {
            println!(
                "calling network for project file, asking to download from DHT: {:?}",
                &job.get_file_name_expected()
            );

            // TODO: Find a way to implement network partition to break up files chunks for parallel network transfer.
            let search_directory = project_file_path
                .parent()
                .expect("Shouldn't be anywhere near root level?");

            // so I need to figure out something about this...
            // TODO - find a way to break out of this if we can't fetch the project file.
            let job = AsRef::<Job>::as_ref(&task);
            let file_name = job.get_file_name_expected().to_string_lossy();

            // TODO: To receive the path or not to modify existing project_file value? I expect both would have the same value?
            let path = client
                .get_file_from_peers(&file_name, search_directory)
                .await
                .map_err(ServerError::NetworkError)?;
            return Ok(path);
        }

        Ok(project_file_path)
    }

    /*
    async fn verify_and_check_render_output_path(
        &self,
        id: &Uuid,
    ) -> Result<PathBuf, async_std::io::Error> {
        // create a output destination for the render image
        let output = self.settings.render_dir.join(&id.to_string());
        async_std::fs::create_dir_all(&output).await?;
        Ok(output)
    }
    */

    // Originally designed to be used to check blender version across network.
    // TODO: Future work - Implement a pattern that
    //  A) Search the network if exact version of blender exist.
    //  B) If the network have similar or newer patches than target version
    //  C) Check local if exact or newer version exist
    //  D) Second to last resort: Download blender from internet
    //  E) Throw error that no blender installation could be fetch or found for this task.
    // #[allow(dead_code)]
    /*
    async fn check_for_blender(&self, version: &Version) -> Result<&Blender, ServerError> {
        // this script below was our internal implementation of handling DHT fallback mode
        // save this for future feature updates
        let mut_manager = self.manager.clone().get_mut().unwrap();

        let blender = match mut_manager.have_blender(version) {
            Some(blend) => blend,
            None => {
                // when I do not have ticket blender version installed - two things will happen here before an error is thrown
                // First, check our internal DHT services to see if any other client on the network have matching version - then fetch it. Install after completion
                // Secondly, download the file online.
                // If we reach here - it is because no other node have matching version, and unable to connect to download url (Internet connectivity most likely).
                // TODO: It would be nice to broadcast everyone else "Hey! I'm download this version, could you wait until I'm done to distribute?"
                panic!("Finish implementing this part");
                /*
                let destination = self.manager.get_install_path();


                    // should also use this to send CmdCommands for network stuff.
                    // where did this client come from?
                    let latest = self
                    .client
                    .get_file_from_peers(&link_name, destination)
                    .await;

                match latest {
                    Ok(path) => {
                        // assumed the file I downloaded is already zipped, proceed with caution on installing.
                        let folder_name = self.manager.get_install_path();
                        let exe =
                        DownloadLink::extract_content(path, folder_name.to_str().unwrap())
                        .expect(
                            "Unable to extract content, More likely a permission issue?",
                        );
                        &Blender::from_executable(exe).expect("Received invalid blender copy!")
                    }
                    Err(e) => {
                        println!(
                            "No client on network is advertising target blender installation! {e:?}"
                        );
                        &self
                        .manager
                        .fetch_blender(&version)
                        .expect("Fail to download blender")
                    }
                }
                */
            }
        };
        Ok(blender)
    }
    */

    // TODO: This will change. We will treat network user as end point of cli interfaces to this app.
    // Received network command.
    // Obsolete. We should not rely on network asking us to render.
    // not yet?
    /*
    async fn handle_job_from_network(&mut self, client: &Controller, event: JobEvent) {
        // with the sqlite connection we can create and establish database struct here.

        match event {
            // on render ticket received, we should store this in the database.
            JobEvent::Render(peer_id_str, mut task) => {
                let peer_id = match PeerId::from_str(&peer_id_str) {
                    Ok(peer_id) => peer_id,
                    Err(e) => {
                        eprintln!("Not a valid peer id! {e:?}");
                        return;
                    }
                };

                if client.public_id.ne(&peer_id) {
                    return;
                }

                // TODO: Does this kick off a background job? How do we let this program continue without hang? Threads?
                if let Err(e) = &self.handle_render(&peer_id, &mut task, &client).await {
                    eprintln!("Received Error! {e:?}");
                }
            }

            JobEvent::ImageCompleted { .. } => {} // ignored since we do not want to capture image?
            // For future impl. we can take advantage about how we can allieve existing job load. E.g. if I'm still rendering 50%, try to send this node the remaining parts?
            JobEvent::TaskComplete => {} // Ignored, we're treated as a client node, waiting for new job request.
            // Remove all ticket with matching job id.
            JobEvent::Remove(job_id) => {
                let task_store = SqliteTaskStore::new(self.db_conn.clone());
                let db = &task_store;
                if let Err(e) = db.delete_job_task(&job_id).await {
                    eprintln!("Unable to remove all ticket with matching job id! {e:?}");
                }
                // Find a way to check and see if we are running any ticket that matches target job_id and stop the blender sequence immediately.
            }
            _ => println!("Unhandle Job Event: {event:?}"),
        }
    }
    */

    // Handle network event (Receiving network messages)
    // async fn handle_net_event(
    //     // TODO: Remove self. Make this not dependent on cli struct (Use caller to send cmd commands)
    //     // &mut self,
    //     // network controller
    //     client: &Controller,
    //     // received network event
    //     event: Event,
    //     // used to interface cli background workers
    //     caller: Sender<ServerCommand>
    // ) -> Result<(), NetworkError> {
    //     match event
    //     Ok(())
    // }

    // Take action from interface. (CLI mode)
    async fn handle_command(
        &mut self,
        client: &NetworkController,
        cmd: ServerCommand,
    ) -> Result<(), NetworkError> {
        // More to come soon. Just making it work for now is bare minimum.
        match cmd {
            ServerCommand::AddTicket(ticket) => {
                let ticket_db = SqliteTicketStore::new(self.db_conn.clone());
                if let Err(e) = ticket_db.add_ticket(ticket).await {
                    eprintln!("Unable to add ticket to database! {e:?}");
                }
            }
            // how does abort works? We'll come back to this later.
            ServerCommand::Abort => {
                // An abort was called. Stop blender.
                todo!("Impl. cancellation token");
            }

            ServerCommand::Fetch(job_id, sender) => {
                // returns a hashset of all render frames from matching job.
                // Inner join tasks inner join renders
                // basically providing basic information to client what frames have been completed.
                let render_db = SqliteRenderStore::new(self.db_conn.clone());
                if let Ok(result) = render_db.find(Some(job_id)).await {
                    if let Err(e) = sender.send(result) {
                        eprintln!("unable to send fetch result! {e:?}");
                    }
                }
            }
            ServerCommand::Start => {
                match Self::start_worker_service(
                    self.db_conn.clone(),
                    self.manager.clone(),
                    &client,
                )
                .await
                {
                    Ok(..) => {
                        todo!("Handle event_receiver here!");
                    }
                    Err(e) => {
                        println!("unable to start worker service! {e:?}");
                        ()
                    }
                }
            }
            ServerCommand::DeleteTicket(id) => {
                let ticket_db = SqliteTicketStore::new(self.db_conn.clone());
                if let Err(e) = ticket_db.delete_ticket(&id).await {
                    eprintln!("Unable to delete ticket from database! {e:?}");
                }
            }
            ServerCommand::CheckBlender(peer_id, zip_file_name) => {
                // ok so we get manager and fetch the package that matches file name asking
                let mut_manager = self.manager.write().await;

                if let Some(path) = mut_manager.check_compressed_by_file_name(&zip_file_name) {
                    let provider = ProviderRule::Custom(zip_file_name, path);
                    if let Err(e) = client.start_providing(&provider).await {
                        eprintln!("Unable to provide files! {e:?}");
                    }
                    // TODO: reply back to the caller
                    todo!("How do I reply back to this peer? {peer_id:?}");
                }
            }
        };
        Ok(())
    }

    async fn start_worker_service(
        db_conn: Pool<Sqlite>,
        manager: Arc<RwLock<BlenderManager>>,
        _controller: &NetworkController,
    ) -> Result<() /*Receiver<BlenderEvent>*/, TicketError> {
        // run the service here.
        let ticket_db = SqliteTicketStore::new(db_conn.clone());

        loop {
            if let Ok(Some(record)) = ticket_db.poll_ticket().await {
                let mut ticket = record.item.clone();

                // Skip this for now. We'll work on DHT at another time.
                // TODO: validate and make sure that we have the files locally stored ready to be used.
                // let project_file = match self.validate_project_file(client, &task).await {
                //     Ok(path) => path,
                //     Err(e) => {
                //         eprintln!("Fail to validate project file! {e:?}");
                //         return;
                //     }
                // };
                // let project_file = task.get_job().get_project_path();

                let version = &ticket.job.blender_version;

                let mut mut_manager = manager.write().await;

                let config = mut_manager.get_config();

                // TODO: I want to find a way to utilize intranet DHT services to fetch installation from other computer node. It wouldn't make a lot of sense re-download the same version from source multiple of times.
                let blender = match config.get_blender(version) {
                    Some(blender) => blender,
                    None => {
                        // Update ticket status to "Error" -> Do not re-run this again until the issue has been resolved.
                        // Server had issue with this job - Send notification broadcast, and delete ticket.
                        if let Err(e) = ticket_db.delete_ticket(&record.id).await {
                            eprintln!("Unable to delete the ticket! {e:?}");
                        }

                        &mut_manager.fetch_blender(version).expect(
                            "Blendfarm must have permission to download and install blender!",
                        )
                        // let (sender, receiver) = mpsc::<>channel();
                        // &controller.
                        // Here, we'd like to try and fetch from client first, before we can download.
                        // &self.manager
                        //     .fetch_blender(&version)
                        //     .map_err(TicketError::Manager)?
                    }
                };

                // we will get to the part of handling receiver, but I wanted to make sure this works so far.
                let _receiver = ticket.render(&blender).await?;
            } else {
                break Ok(());
            }
        }
    }
}

#[async_trait]
impl BlendFarm for Server {
    /*
        Some thoughts:
        The Cli App mode should be stateless, e.g. no Idle state. The services that BlendFarm runs on should utilize the necessary components to run blender from network request.
        The Cli must have a switch to listen for server connection to become state machines. (TODO: E.g. provide IP and Port)
    */

    /// This program will run into this following state machine:
    /// It will continue to poll ticket from the database and work on the given assignments.
    /// The ticket will be reflected by the host machine once available, and other peers can request tasks, if they exhaust their ticket queue.
    /// Once exhausted all pending ticket, send out RequestTicket signal and send to newly discover node.
    /// The background network services will update and monitor the database connection, as well as governs the ticket lifetime handlers.
    ///     E.g. A job cancellation notice should terminate ongoing ticket jobs. Needs a way to interface ongoing thread and abort before resuming next task.
    /// Future work: The node can be in a "Paused" state, given under circumstances, that it should await for host's further instructions.
    ///     E.g. Downloading blender in background.
    /// The run command will launch two processes. One process will monitor and receive Blender activity.
    /// The other process handles network events.
    async fn run(
        mut self,
        client: NetworkController,
        mut event_receiver: Receiver<Event>,
    ) -> Result<(), BlendFarmError> {
        // I need to find a way to safely notify the background to stop in case the job was deleted from host machine.
        // we will have one thread to process blender and queue, but I must have access to database.
        // where is this event suppose to be used for?
        let (event, mut command) = mpsc::channel(32);

        // background thread to handle blender invocation
        // let blender_controller = client.clone();

        // if we exit early, how do we restart this service?
        let ticket_db = SqliteTicketStore::new(self.db_conn.clone());
        // let render_db = SqliteRenderStore::new(self.db_conn.clone());
        // let spec = ComputerSpec::new();

        // spawn(async move {
        //     // let id = blender_controller.public_id;
        //     let task_store = SqliteTicketStore::new(self.db_conn);

        //     // loop until we have no more ticket left to work on.
        //     while let Ok(receiver) = &mut ticket_db.poll_ticket().await {
        //         while let Some(message) = receiver.recv().await {
        //                 // if receiver.
        //                 // BlenderEvent::
        //                 // match message {
        //                 //     BlenderEvent::Quit
        //                 // }
        //                 print!("Processing tickets: {:?}", &message);
        //         }
        //         break;
        //     }

        //     // Once we've exhausted all of theticket here, we should send out Request ticket message.
        //     blender_controller.send_server_status(ServerEvent::Idle).await;
        // });

        let public_addr = Multiaddr::empty();

        let _event =
            Server::start_worker_service(self.db_conn.clone(), self.manager.clone(), &client)
                .await
                .map_err(BlendFarmError::TicketError);

        // Process pending inputs commands from foreign function interface
        loop {
            select! {
                pending_event = event_receiver.recv() => match pending_event {
                    Some(network_event) => match network_event {
                        Event::Discovered( _, peer_addr ) => {
                            println!("Peer Discovered: {}", &peer_addr);

                            // Perform a check. If we have exhausted our ticket queue, we should send this discover peer a RequestTicket message.
                            if let Ok(Some(remains)) = ticket_db.list_tickets().await {
                                if remains.len().eq(&0) {
                                    // now we will just simply ask
                                    let local_addr = &client.multiaddr;
                                    println!("Sending discovered peer a request ticket message.");
                                    client.send_peer_message(&peer_addr, ServerEvent::RequestTicket(local_addr.clone())).await;
                                }
                            }

                            // TODO: See if we can avoid instantiating a new Computer spec, cache this somewhere, in a struct
                            let spec = ComputerSpec::new();
                            println!("Sending discovered peer a online status message.");
                            // We'll say I'm online instead of requesting ticket.
                            client.send_peer_message(&peer_addr, ServerEvent::Online(public_addr.clone(), spec)).await;
                        }
                        Event::JobUpdate(job_event) => {
                            println!("Received Job Event: {job_event:?}")
                            // caller
                            //self.handle_job_from_network(client, job_event).await,
                        },
                        Event::InboundRequest { request, channel } => {
                            Self::handle_inbound_request(&client, request, channel).await
                        }
                        Event::ServerStatus(event) => {
                            match event {
                                ServerEvent::Joined(peer_id) => {
                                    println!("A peer [{:?}] has joined the channel", peer_id);
                                },
                                ServerEvent::RemoveJob(job_id) => {
                                    if let Err(e) = ticket_db.delete_job_ticket(&job_id).await {
                                        eprintln!("Fail to remove ticket with matching job id {job_id} | {e:?}");
                                    }
                                },
                                ServerEvent::NewTickets(ticket) => {
                                    if let Err(e) = ticket_db.add_ticket(ticket).await {
                                        eprintln!("Fail to add new ticket to database! {e:?}");
                                    }
                                },
                                ServerEvent::RequestTicket(peer_addr) => {
                                    // From a service point of view, should we be smart enough to allow this node to distribute pending tickets?
                                    // TODO Make this display via verbose/debug options
                                    println!("Peer [{peer_addr}] is requesting a ticket.");
                                    // if we do not have a job, then we can request ticke to this target peer_id.
                                    if let Ok(query) = ticket_db.list_tickets().await {
                                        if let Some(col) = query {
                                            if col.len().gt(&3) {
                                                continue;
                                            }
                                        }
                                    }

                                    println!("I should contact this peer_addr and send them a new ticket.")
                                    // Ok so if we dial, what are we doing here?
                                    // if let Err(e) = client.dial(&peer_addr).await {
                                    //     eprintln!("Unable to dial! {e:?}");
                                    // }
                                },
                                ServerEvent::Online(peer_addr, spec) => {
                                    // peer connected with specs.
                                    // Once a computer becomes online, do nothing?

                                    println!("Peer connected with specs provided : {peer_addr:?}\n{spec:?}");
                                    // if we are not connected to host, connect to this one. await further instructions.
                                    // TODO: See where my multiaddr went?
                                    // self.host = Some((PeerIdStr::from(peer_id), multiaddr));

                                    // let public_ip = client.public_id.to_base58();
                                    // let mut machine = Machine::new();
                                    // let computer_spec = ComputerSpec::new(&mut machine);
                                    // let status = NodeEvent::Hello(public_ip, computer_spec);
                                    // client.send_node_status(status).await;
                                }
                                ServerEvent::Disconnected { peer_id, reason } => match reason {
                                    Some(err) => {
                                        // Reporting that we lost connection to peer_id by a connection IO error
                                        println!("Peer Disconnected with reason [{peer_id:?}] {err}");
                                        // what shall the server ever do? Do we care? No?
                                    }
                                    None => println!("Peer Disconnected without reason! [{peer_id:?}]"),
                                },
                                ServerEvent::BlenderStatus(_blender_event) => {
                                    // println!("[Blender Status] {blender_event:?}");
                                    // probably doesn't matter, but shouldn't spam the network with this info yet...
                                },
                                // ServerEvent::Idle => {
                                //     eprintln!("A node has entered idle state... We should probably give that node some job to work on...");
                                // }
                                ServerEvent::Rendering(_) => {
                                    // We can ignore this, server aren't suppose to care about what other server rendering status looks like.
                                }
                                ServerEvent::RequestJobInfo(job_id) => {
                                    // we received a job info request. Check our internal data and reply back with job info.
                                    let render_db = SqliteRenderStore::new(self.db_conn.clone());
                                    let result = render_db.find(Some(job_id)).await;

                                    if let Ok(jobs) = result {
                                        let data = serde_json::to_string(&jobs);
                                        let _ = dbg!(data);
                                        // TODO: How can I dial back the requestor who ask for this job info?
                                        // let server_event = ServerEvent::
                                        // client.send_server_status(server_event).await;
                                    }
                                }
                            }
                        }
                        _ => println!("[Server] Unhandled event received from network: {event:?}"),
                    },
                    None => {
                        // pipe was closed, begin shut down.
                        // TODO: See how we can gracefully shutdown?
                        break Ok(())
                    }

                },
                // can I send this command to net event?
                msg = command.recv() => match msg {
                    Some(cmd) => self.handle_command(&client, cmd).await?,
                    None => {
                        println!("None was received, continue?");
                        break Ok(())
                    },
                },
            }
        }
    }
}
