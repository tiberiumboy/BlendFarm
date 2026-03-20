/*
Have a look into TUI for CLI status display window to show user entertainment on screen
https://docs.rs/tui/latest/tui/

Feature request:
    - See how we can treat this application process as service mode so that it can be initialize and start on machine reboot?
    - receive command to properly reboot computer when possible?
*/
use super::blend_farm::BlendFarm;
use crate::domains::ticket_store::{TicketError, TicketStore};
use crate::models::computer_spec::ComputerSpec;
use crate::network::PeerIdString;
use crate::network::message::{self, Event, NetworkError};
use crate::network::provider_rule::ProviderRule;
use crate::services::app_context::AppContext;
use crate::services::data_store::sqlite_renders_store::SqliteRenderStore;
use crate::services::data_store::sqlite_ticket_store::SqliteTicketStore;
use crate::{
    models::{
        job::Job,
        server_setting::ServerSetting,
        ticket::Ticket,
    },
    network::controller::Controller as NetworkController,
};
use blender::blender::{Blender, Frame, Manager as BlenderManager, ManagerError};
use blender::models::event::BlenderEvent;
use semver::Version;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::collections::HashSet;
use std::path::PathBuf;
use tauri::async_runtime::Receiver;
use thiserror::Error;
use tokio::sync::mpsc::{self, Sender};
use tokio::{select, spawn};
use uuid::Uuid;

// this is invocation commands. Signal to start, stop, fetch blender information and relative info.
enum ServerCommand {
    Start,
    AddTask(Ticket),
    DeleteTask(Uuid),
    CheckBlender(String),  // Name of the blender in compressed package enum. (e.g. "blender-5.0.0-linux-x64.tar.xz")
    // this function seems confusing. Refine this a bit letter.
    Fetch(Sender<HashSet<Frame, PathBuf>>),
    Abort,
}

// Must be serializable to send data across network
// issue with this is that this cannot be convert into Encode,Decode by bincode. Instead we'll have to
#[derive(Debug, Serialize, Deserialize)]
pub enum ServerEvent {
    Online(PeerIdString, ComputerSpec),
    // Network received a disconnected signal from peer_id.
    Disconnected {
        peer_id: PeerIdString,
        reason: Option<String>,
    },
    Rendering(Uuid),
    DownloadingBlender(Version),
    BlenderStatus(BlenderEvent),
    Idle,   // waiting for task
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
    TaskError(#[from] TicketError)
}

/// The behaviour described in the Cli App can be summarize below:
/// When running with listening server, client will spin a thread to listen for network messages.
///     Cli as a listening server can accept Request Task from available host.
///     The host can ask you about your task's progress, and how many image you've completed.
///     Additionally, the host may also request the images from you.
/// When running in pure cli mode, you can ask to fetch information and create new task from local machine.
/// This will let cli mode run the job in batch mode, customized for your experience.
/// and simply closes out.
/// This will be useful for blender add-on interface, we want to be able to invoke client/host commands from blender application, as an alternative solution.
pub struct Server {
    manager: BlenderManager,

    // database connection
    db_conn: Pool<Sqlite>,

    // config
    settings: ServerSetting,

    // current server specs
    pub spec: ComputerSpec,
}

// cli app should really be a stateless machine. A listener would just receive order from the network and proceed the task given queued.
// This program should close after completing the task queue, in non-listening mode
impl Server {

    pub(crate) fn new(context: AppContext, db: &Pool<Sqlite>) -> Self {
        Self {
            settings: context.settings,
            manager: context.manager,
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

            let providers = client.get_providers(job.get_file_name_expected().clone()).await;


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

    async fn verify_and_check_render_output_path(
        &self,
        id: &Uuid,
    ) -> Result<PathBuf, async_std::io::Error> {
        // create a output destination for the render image
        let output = self.settings.render_dir.join(&id.to_string());
        async_std::fs::create_dir_all(&output).await?;
        Ok(output)
    }

    // Originally designed to be used to check blender version across network.
    // TODO: Future work - Implement a pattern that
    //  A) Search the network if exact version of blender exist.
    //  B) If the network have similar or newer patches than target version
    //  C) Check local if exact or newer version exist
    //  D) Second to last resort: Download blender from internet
    //  E) Throw error that no blender installation could be fetch or found for this task.
    #[allow(dead_code)]
    async fn check_for_blender(&self, version: &Version) -> Result<&Blender, ServerError> {
        // this script below was our internal implementation of handling DHT fallback mode
        // save this for future feature updates
        let blender = match self.manager.have_blender(version) {
            Some(blend) => blend,
            None => {
                // when I do not have task blender version installed - two things will happen here before an error is thrown
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

    // TODO: This will change. We will treat network user as end point of cli interfaces to this app.
    // Received network command. 
    // Obsolete. We should not rely on network asking us to render.
    // not yet?
    /* 
    async fn handle_job_from_network(&mut self, client: &Controller, event: JobEvent) {
        // with the sqlite connection we can create and establish database struct here.

        match event {
            // on render task received, we should store this in the database.
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
            // Remove all task with matching job id.
            JobEvent::Remove(job_id) => {
                let task_store = SqliteTaskStore::new(self.db_conn.clone());
                let db = &task_store;
                if let Err(e) = db.delete_job_task(&job_id).await {
                    eprintln!("Unable to remove all task with matching job id! {e:?}");
                }
                // Find a way to check and see if we are running any task that matches target job_id and stop the blender sequence immediately.
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
            ServerCommand::AddTask(ticket) => {
                let ticket_db = SqliteTicketStore::new(self.db_conn.clone());
                ticket_db.add_task(ticket).await; 
            },
            // how does abort works? We'll come back to this later.
            ServerCommand::Abort => {
                // An abort was called. Stop blender.
                
            },

            ServerCommand::Fetch(sender) => todo!(),
            ServerCommand::Start => {
                self.process_tickets(&client);
                ()
            },
            ServerCommand::DeleteTask(uuid) => todo!(),
            // TODO: Consider adding the peer_id that called this function
            ServerCommand::CheckBlender(zip_file_name) => {
                // ok so we get manager and fetch the package that matches file name asking
                if let Some(path) = self.manager.check_compressed_by_file_name(&zip_file_name) {
                    let provider = ProviderRule::Custom(zip_file_name, path);
                    client.start_providing(&provider).await;
                    // TODO: reply back to the caller
                    
                }
            },
        };
        Ok(())
    }

    async fn process_tickets(
        &mut self,
        controller: &NetworkController,
    ) -> Result<Receiver<BlenderEvent>, TicketError> {
        // run the service here.
                let ticket_db = SqliteTicketStore::new(self.db_conn.clone());
                // let render_db = SqliteRenderStore::new(self.db_conn.clone());

                loop {
                    select! {
                        pending_task = ticket_db.poll_ticket() => match pending_task {
                            Ok(query) => match query {
                                Some(record) => {
                                    let mut ticket = record.item;
                                    
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
                                    // TODO: I want to find a way to utilize intranet DHT services to fetch installation from other computer node. It wouldn't make a lot of sense re-download the same version from source multiple of times.
                                    let blender = match self.manager.have_blender(version) {
                                        Some(blender) => blender,
                                        None => {
                                            &controller.
                                            // Here, we'd like to try and fetch from client first, before we can download.
                                            // &self.manager
                                            //     .fetch_blender(&version)
                                            //     .map_err(TicketError::Manager)?
                                        }
                                    };

                                    // we will get to the part of handling receiver, but I wanted to make sure this works so far.
                                    let _receiver = ticket.render(&blender).await?;
                                    ()
                                }
                                None => (),
                            },
                            Err(e) => ()
                        }
                    }
                }
        
    }
}

#[async_trait::async_trait]
impl BlendFarm for Server {
    /*
        Some thoughts:
        The Cli App mode should be stateless, e.g. no Idle state. The services that BlendFarm runs on should utilize the necessary components to run blender from network request.
        The Cli must have a switch to listen for server connection to become state machines. (TODO: E.g. provide IP and Port)
    */

    /// This program will run into this following state machine:
    /// It will continue to poll task from the database and work on the given assignments.
    /// The task will be reflected by the host machine once available, and other peers can request tasks, if they're idle.
    /// Once exhausted all pending task, this node will send out one RequestTask message to the network and remain idle.
    /// It will also send discovered node a RequestTask as well.
    /// The background network services will update and monitor the database connection, as well as governs the task lifetime handlers.
    ///     E.g. A job cancellation notice should terminate ongoing task jobs. Needs a way to interface ongoing thread and abort before resuming next task.
    /// Future work: The node can be in a "Paused" state, given under circumstances, that it should await for host's further instructions.
    ///     E.g. Downloading blender in background.
    /// The run command will launch two processes. One process will monitor and receive Blender activity.
    /// The other process handles network events.
    async fn run(
        mut self,
        mut client: NetworkController,
        mut event_receiver: Receiver<Event>,
    ) -> Result<(), NetworkError> {

        // I need to find a way to safely notify the background to stop in case the job was deleted from host machine.
        // we will have one thread to process blender and queue, but I must have access to database.
        let (event, mut command) = mpsc::channel(32);
        
        // background thread to handle blender invocation
        let blender_controller = client.clone();
        
        // if we exit early, how do we restart this service?
        let task_db = SqliteTicketStore::new(self.db_conn.clone());
        let render_db = SqliteRenderStore::new(self.db_conn.clone());
        let spec = ComputerSpec::new();

        spawn(async move {
            let mut has_started = false;
            let id = blender_controller.public_id;
            let task_store = SqliteTicketStore::new(self.db_conn);
            // loop until we have no more task left to work on.
            loop {
                select! {
                    blender_event = self.process_task(&blender_controller).await => self.handle_blender_event(blender_event),                
                }
            }

            // Once we've exhausted all of the task here, we should send out Request Task message.
            blender_controller.send_node_status(ServerEvent::Idle).await;
        });

        // Process commands inputs
        // This will be moved somewhere else.
        loop {
            select! {
                pending_event = event_receiver.recv() => match pending_event {
                    Some(network_event) => match network_event {
                        Event::Discovered(peer_id, multiaddr) => {
                            // I don't think I need to care about this?
                            println!("Discovered peer! {peer_id} | {multiaddr}");
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
                                ServerEvent::Online(peer_id, spec) => {
                                    // peer connected with specs.

                                    peer_id
                                    println!("Peer connected with specs provided : {peer_id:?}\n{spec:?}");
                                    // if we are not connected to host, connect to this one. await further instructions.
                                    // TODO: See where my multiaddr went?
                                    // self.host = Some((PeerIdStr::from(peer_id), multiaddr));
                                    
                                    // let public_ip = client.public_id.to_base58();
                                    // let mut machine = Machine::new();
                                    // let computer_spec = ComputerSpec::new(&mut machine);
                                    // let status = NodeEvent::Hello(public_ip, computer_spec);
                                    // client.send_node_status(status).await;
                                }
                                // ServerEvent::Disconnected { peer_id, reason } => match reason {
                                //     Some(err) => {
                                //         // Reporting that we lost connection to peer_id by a connection IO error 
                                //         println!("Peer Disconnected with reason [{peer_id:?}] {err}");
                                //         // what shall the server ever do? Do we care? No?
                                //     }
                                //     None => println!("Peer Disconnected without reason! [{peer_id:?}]"),
                                // },
                                ServerEvent::BlenderStatus(_blender_event) => {
                                    // println!("[Blender Status] {blender_event:?}");
                                    // probably doesn't matter, but shouldn't spam the network with this info yet...
                                },
                                ServerEvent::Idle => {
                                    eprintln!("A node has entered idle state... We should probably give that node some job to work on...");
                                }
                            }
                        }
                        _ => println!("[Server] Unhandled event received from network: {event:?}"),
                    },
                    None => {
                        // pipe was closed, begin shut down.
                        // TODO: See how we can gracefully shutdown?
                        ()
                    }
        
                },
                // can I send this command to net event?
                msg = command.recv() => match msg {
                    Some(cmd) => Self::handle_command(&client, cmd).await?,
                    None => (),
                },
            }
        }
    }
}
