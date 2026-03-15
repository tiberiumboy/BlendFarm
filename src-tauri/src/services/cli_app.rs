/*
Have a look into TUI for CLI status display window to show user entertainment on screen
https://docs.rs/tui/latest/tui/

Feature request:
    - See how we can treat this application process as service mode so that it can be initialize and start on machine reboot?
    - receive command to properly reboot computer when possible?
*/
use super::blend_farm::BlendFarm;
use crate::domains::render_store::RenderStore;
use crate::domains::task_store::TaskError;
use crate::models::render_info::NewRenderInfoDto;
use crate::models::with_id::WithId;
use crate::network::message::{self, ChannelStatus, Event, NetworkError, NodeEvent};
use crate::network::provider_rule::ProviderRule;
use crate::services::app_context::AppContext;
use crate::services::data_store::sqlite_renders_store::SqliteRenderStore;
use crate::services::data_store::sqlite_task_store::SqliteTaskStore;
use crate::{
    domains::{job_store::JobError, task_store::TaskStore},
    models::{
        job::{Job, JobEvent},
        server_setting::ServerSetting,
        task::Task,
    },
    network::controller::Controller,
};
use async_std::task::spawn;
use blender::blend_file::BlendFile;
use blender::blender::{Args, Blender, Manager as BlenderManager, ManagerError};
use blender::models::event::BlenderEvent;
use libp2p::{Multiaddr, PeerId};
use semver::Version;
use sqlx::{Pool, Sqlite};
use tauri::async_runtime::Receiver;
use tokio::task::JoinHandle;
use std::{path::PathBuf, str::FromStr};
use thiserror::Error;
use tokio::sync::mpsc::{self, Sender};
use tokio::select;
use uuid::Uuid;

// TODO: What was this for?
#[allow(dead_code)]
enum CmdCommand {
    // TODO: See where this can be used?
    Render(Task, Sender<BlenderEvent>),
    Dial(PeerId, Multiaddr),
    RequestTask, // calls to host for more task.
}

#[derive(Debug, Error)]
enum CliError {
    #[error("Encounter an network error! \n{0:}")]
    NetworkError(#[from] message::NetworkError),
    #[error("Encounter an IO error! \n{0}")]
    Io(#[from] async_std::io::Error),
    #[error("Manager Error: {0}")]
    ManagerError(#[from] ManagerError),
}

pub struct CliApp {
    manager: BlenderManager,

    // database
    task_store: SqliteTaskStore,
    render_store: SqliteRenderStore,

    // config
    settings: ServerSetting,

    // The idea behind this is to let the network manager aware that the client side of the app is busy working on current task.
    // it would be nice to receive information and notification about this current client status somehow.
    // Could I use PhantomData to hold Task Object type?
    host: Option<(PeerId, Multiaddr)>, // instead of this, we should hold task_handler. That way, we can abort it when we receive the invocation to do so.

    // to see if there's any job running.
    handler: Option<JoinHandle<()>>,
}

impl CliApp {

    // This function sends out a command request to the other thread to launch blender and render the given task.
    // In return, we should try to return the JohnHandler<()> so that we can gracefully abort the task.
    async fn subscribe_to_render_job(&mut self, task: WithId<Task,Uuid>, event: &Sender<CmdCommand>, controller: &mut Controller, render_db: &SqliteRenderStore) {
        // why did this method get invoked twice?
        // This have code smells. I'm sending a request to another thread to start the rendering job, but that allows me to continue to listen for server updates.
        // if the host replied to cancel specific job, I must be able to acknowledge the request and act upon immediately without delay.
        // TODO: Display this under certain verbosity
        println!("Begin task {:?}!", &task.id);
        let (sender, mut receiver) = mpsc::channel(32);
        let job_id_ref: &Uuid = AsRef::as_ref(&task);
        let job_id = job_id_ref.to_owned();
        let cmd = CmdCommand::Render(task.item, sender);
        if let Err(e) = event.send(cmd).await {
            // TODO: Display this under certain verbosity
            eprintln!("Fail to send backend service render request! {e:?}");
        }

        // begin streaming progress to network protocols.
        loop {
            select! {
                event = receiver.recv() => match event {
                    Some(event) => {
                        match event {
                            // TODO: Find ways to print this via verbose command
                            BlenderEvent::Log(log) => println!("{log}"),
                            // TODO: Find ways to print this via verbose command
                            BlenderEvent::Warning(warn) => println!("{warn}"),
                            // TODO: Find ways to print this via verbose command
                            // maybe it would be nice to send this network message back to network?
                            BlenderEvent::Rendering { current, total } => {
                                println!("Rendering {current} out of {total}")

                            },
                            BlenderEvent::Completed { result, frame } => {
                                let render_info = NewRenderInfoDto::new(job_id.clone(), frame, &result );
                                // TODO: Find ways to print this via verbose command
                                if let Err(e) = &render_db.create_renders(render_info).await {
                                    eprintln!("Fail to create a new render entry to the database! {e:?}");
                                }
                                // sends a 
                                let event = JobEvent::ImageCompleted {
                                    job_id: job_id.clone(),
                                    frame,
                                    file_name: result.to_str().unwrap().to_owned()      
                                };
                                controller.send_job_event(event).await;
                            },
                            // receiving unhandled event for getting blender version and commit hash value?
                            BlenderEvent::Unhandled(e) => {
                                // Blender 4.3.2 (hash 32f5fdce0a0a built 2024-12-17 02:14:25)
                                eprintln!("{e:?}");
                            },
                            BlenderEvent::Exit => break,
                            BlenderEvent::Error(e) => {
                                eprintln!("Received Blender Error: {e:?}");
                            },
                        }
                    },
                    None => {
                        // TODO: Find a way to display verbosity via switch
                        // eprintln!("Received None from Blender loop! Breaking");
                        break
                    }
                }
            }
        }
    }

    // we could simplify this design by just asking for the database info?
    pub(crate) fn new(
        context: AppContext,
        db: &Pool<Sqlite>
    ) -> Self {

        let task_store = SqliteTaskStore::new(db.clone());
        let render_store = SqliteRenderStore::new(db.clone());

        Self {
            settings: context.settings,
            manager: context.manager,
            task_store,
            render_store,
            handler: None,
            // TODO: why do I need to care about this?
            host: None, // no task assigned yet
        }
    }

    // This function will ensure the directory will exist, and return the path to that given directory.
    // It will remain valid unless directory or parent above is removed during runtime.
    async fn generate_temp_project_task_directory(
        settings: &ServerSetting,
        task: &Task,
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
        client: &mut Controller,
        task: &Task,
    ) -> Result<PathBuf, CliError> {
        let id = AsRef::<Uuid>::as_ref(&task);
        let project_file_path =
            CliApp::generate_temp_project_task_directory(&self.settings, &task, &id.to_string())
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
                .map_err(CliError::NetworkError)?;
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

    // TODO: See where this was originally used, and see if we can remove this.
    #[allow(dead_code)]
    async fn check_for_blender(&self, version: &Version) -> Result<&Blender, CliError> {
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

    // TODO: Refactor this!
    // TODO: Rewrite this to meet Single responsibility principle.
    // How do I abort the job? -> That's the neat part! You don't! Delete the job+task entry from the database, and notify client to halt if running deleted jobs.
    /// Invokes the render job. The task needs to be mutable for frame deque.
    async fn render_task(
        &mut self,
        client: &mut Controller,
        task: &mut Task,
        sender: &mut Sender<BlenderEvent>,
    ) -> Result<(), CliError> {
        // why do I need the job info?
        let job = AsRef::<Job>::as_ref(&task);
        let blend_file = AsRef::<BlendFile>::as_ref(&job);
        let version = job.as_ref();

        // for now, let's skip this part and continue on. We don't have DHT setup, but I want to make sure cli does actually render once we get the file share situation straighten out.
        // TODO: Find a way to get the file share working across network.
        // let project_file = self.validate_project_file(client, &task).await?;
        // self.check_for_blender()?;

        // get blender executables
        let blender = self
            .manager
            .fetch_blender(version)
            .map_err(CliError::ManagerError)?;

        // get the ID of the task for parent directory name
        let id = AsRef::<Uuid>::as_ref(&task);
        
        // Generate a new local destination path. Overriding scene's path to valid path location.
        // TODO: This will throw an error if the directory already exist?
        let output = self
            .verify_and_check_render_output_path(id)
            .await
            .map_err(CliError::Io)?;

        let args = Args::new(blend_file.clone(),output, task.start, task.end);
        
        // run the job!
        match blender.render(args).await.map_err(TaskError::BlenderError) {
            Ok(rx) => loop {
                match rx.recv() {
                    Ok(status) => {
                        // SHould look into a better way to write this so that we can handle loop better for blender process....
                        // Somehow, receiver was closed?
                        match &status {
                            BlenderEvent::Error(..) => {
                                sender
                                    .send(status)
                                    .await
                                    .expect("Channel should not be closed");
                                // make sure to break out of this loop!
                                break;
                            }
                            _ => sender
                                .send(status)
                                .await
                                .expect("Channel should not be closed"),
                        }
                    }
                    Err(e) => {
                        let event = BlenderEvent::Error(e.to_string());
                        if let Err(c) = sender.send(event).await {
                            eprintln!(
                                "Unable to send error event over clseod channel: {c:?}\n{e:?}"
                            );
                        }
                        break;
                    }
                }
            },
            Err(e) => {
                let err = JobError::TaskError(e);
                client.send_job_event(JobEvent::Error(err.to_string())).await;
            }
        };

        Ok(())
    }

    async fn handle_job_from_network(&mut self, client: &mut Controller, event: JobEvent) {
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

                // Skip this for now. We'll work on DHT at another time.
                // let project_file = match self.validate_project_file(client, &task).await {
                //     Ok(path) => path,
                //     Err(e) => {
                //         eprintln!("Fail to validate project file! {e:?}");
                //         return;
                //     }
                // };
                // let project_file = task.get_job().get_project_path();

                // scope containing using self. Need to close at the end of the scope for other method to use it as mutable state.
                // do we need this right now?
                // Need to make sure no other node work the same job here.
                if let Err(e) = &self.task_store.add_task(task.clone()).await {
                    println!("Unable to add task! {e:?}");
                }
            

                // println!("Begin printing task at this level!");
                // let blend = match &self.manager.fetch_blender(&task.get_job().get_version()) {
                //     Ok(result) => result,
                //     Err(e) => {
                //         eprintln!("problem downloading blender! {e:?}");
                //         return;
                //     }
                // };

                let (mut sender, mut receiver) = mpsc::channel(32);
                let job_id = AsRef::<Uuid>::as_ref(&task).clone();

                match self.render_task(client, &mut task, &mut sender).await {
                    Ok(()) => {
                        println!("task completed!");
                    }
                    Err(e) => {
                        eprintln!("Error rendering task! {e:?}");
                    }
                };

                loop {
                    match receiver.blocking_recv().unwrap_or(BlenderEvent::Error(
                        "Client receiver was closed. Perhaps something happen to the host?"
                            .to_owned(),
                    )) {
                        BlenderEvent::Log(log) => {
                            println!("[LOG] {log}");
                        }
                        BlenderEvent::Warning(warn) => {
                            eprintln!("[WARN] {warn}");
                        }
                        BlenderEvent::Rendering { current, total } => {
                            println!("[LOG] Rendering {current} out of {total}...");
                        }
                        BlenderEvent::Completed { frame, result } => {
                            println!("Image completed!");
                            let provider_rule = ProviderRule::Default(result);
                            if let Err(e) = client.start_providing(&provider_rule).await {
                                eprintln!("Unable to provide completed render image! {e:?}");
                            }

                            match provider_rule.get_file_name() {
                                Some(file_name) => {
                                    let job_event = JobEvent::ImageCompleted {
                                        job_id,
                                        frame,
                                        file_name: file_name.to_str().unwrap().to_string(),
                                    };
                                    client.send_job_event(job_event).await;
                                }
                                None => {
                                    eprintln!(
                                        "Fail to get file name from provider rule - Did we get the file name incorrectly somehow?"
                                    );
                                }
                            };
                        }
                        BlenderEvent::Unhandled(unk) => {
                            eprintln!("An unhandled blender event received: {unk}")
                        }
                        BlenderEvent::Exit => break,
                        BlenderEvent::Error(e) => {
                            eprintln!("Blender error event received! \n{e}");
                        }
                    }
                }
            }

            JobEvent::ImageCompleted { .. } => {} // ignored since we do not want to capture image?
            // For future impl. we can take advantage about how we can allieve existing job load. E.g. if I'm still rendering 50%, try to send this node the remaining parts?
            JobEvent::TaskComplete => {} // Ignored, we're treated as a client node, waiting for new job request.
            // Remove all task with matching job id.
            JobEvent::Remove(job_id) => {
                let db = &self.task_store;
                if let Err(e) = db.delete_job_task(&job_id).await {
                    eprintln!("Unable to remove all task with matching job id! {e:?}");
                }
                // Find a way to check and see if we are running any task that matches target job_id and stop the blender sequence immediately.
            }
            _ => println!("Unhandle Job Event: {event:?}"),
        }
    }

    // Handle network event (From network as user to operate this)
    async fn handle_net_event(&mut self, client: &mut Controller, event: Event) {
        match event {
            // once we discover a peer, let's dial that peer.
            Event::Discovered(peer_id, multiaddr) => {
                if self.host.is_none() {
                    if let Err(e) = client.dial(&peer_id, &multiaddr).await {
                        eprintln!("Fail to dial! {e:?}");
                    }

                    self.host = Some((peer_id, multiaddr));
                }
            }
            Event::JobUpdate(job_event) => self.handle_job_from_network(client, job_event).await,
            Event::InboundRequest { request, channel } => {
                self.handle_inbound_request(client, request, channel).await
            }
            Event::NodeStatus(event) => {
                match event {
                    NodeEvent::Hello(peer_id, spec) => {
                        // peer connected with specs.
                        println!("Peer connected with specs provided : {peer_id:?}\n{spec:?}");
                        // if we are not connected to host, connect to this one. await further instructions.
                        // TODO: See where my multiaddr went?
                        // self.host = Some((PeerIdStr::from(peer_id), multiaddr));
                        todo!("assign host, figure out where my multiaddr went");

                        // let public_ip = client.public_id.to_base58();
                        // let mut machine = Machine::new();
                        // let computer_spec = ComputerSpec::new(&mut machine);
                        // let status = NodeEvent::Hello(public_ip, computer_spec);
                        // client.send_node_status(status).await;
                    }
                    NodeEvent::Disconnected { peer_id, reason } => match reason {
                        Some(err) => {
                            println!("Peer Disconnected with reason [{peer_id:?}] {err}");
                        }
                        None => println!("Peer Disconnected without reason! [{peer_id:?}]"),
                    },
                    NodeEvent::BlenderStatus(_blender_event) => {
                        // println!("[Blender Status] {blender_event:?}");
                        // probably doesn't matter, but shouldn't spam the network with this info yet...
                    }
                }
            }
            Event::Channel(channel_status) => match channel_status {
                ChannelStatus::Joined(peer_id, topic) => {
                    // if we are idle, we should send this peer a RequestTask message.
                    // Hello peer_id, can I request a task from you?
                },
                ChannelStatus::Disconnected(peer_id, _) => {
                    // Oh no, this peer disconnected! what shall we ever do!?
                    eprintln!("TODO: See if we need this conditional branch?");
                }
            }
            _ => println!("[CLI] Unhandled event from network: {event:?}"),
        }
    }

    async fn handle_command(&mut self, client: &mut Controller, cmd: CmdCommand) {
        match cmd {
            CmdCommand::Dial(peer_id, addr) => match client.dial(&peer_id, &addr).await {
                Ok(_) => self.host = Some((peer_id, addr)),
                Err(e) => eprintln!("{e:?}"),
            },

            CmdCommand::Render(mut task, mut sender) => {
                // TODO: We should find a way to mark this node currently busy so we should unsubscribe any pending new jobs if possible?
                // mutate this struct to skip listening for any new jobs.
                // proceed to render the task.
                match self.render_task(client, &mut task, &mut sender).await {
                    Ok(_) => {
                        // here we should send successful result?
                        println!("Successfully rendered task!");
                    }
                    Err(e) => {
                        let event = JobEvent::Failed(e.to_string());
                        client.send_job_event(event).await;
                    }
                }
            }

            CmdCommand::RequestTask => {
                // or at least have this node look into job history and start working on jobs that are not completed yet.
                let peer_id = client.public_id.to_base58();
                let event = JobEvent::RequestTask(peer_id);
                client.send_job_event(event).await;
            }
        }
    }
}

#[async_trait::async_trait]
impl BlendFarm for CliApp {

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
    async fn run(
        mut self,
        mut client: Controller,
        mut event_receiver: Receiver<Event>,
    ) -> Result<(), NetworkError> {
        // I need to find a way to safely notify the background to stop in case the job was deleted from host machine.
        // we will have one thread to process blender and queue, but I must have access to database.

        let (event, mut command) = mpsc::channel(32);
        let taskdb = self.task_store;
        let render_db = self.render_store;

        let mut alter_controller = client.clone();

        // background thread to handle blender invocation
        // So this is where we can say that Cli is a state machine.
        // TODO: Return the JoinHandler<()> for this thread. Once we go through the tauri_app we'll update this trait.
        let _worker_handler = spawn(async move {

            // there are two loops? break the loops up
            loop {
                // get reference to task database
                // let db = taskdb.write().await;
                let db = taskdb;

                // TODO: think I have too many nested conditions here? Is it possible to break apart this component into smaller snippet
                // Yes it's always possible to break this up. I don't think we need to repeatively ask the host for requesting task.
                // The plan is 
                //      A) break up the responsibility. 
                //      B) Cli should work on pending task. Once exhausted all queue - Send RequestTask out. 
                //      C) Also Send RequestTask out to newly discovered node.
                match db.poll_task().await {
                    // if we have a pending task.
                    Ok(result) => {
                        match result {
                            // this begins the render job.
                            Some(task_record) => {
                                // TODO: Future work Add a handler hook. 
                                // Update itself, and assign a job handler.
                                self.subscribe_to_render_job(task_record, &event, &mut alter_controller, &render_db).await;
                            }
                            None => {
                                if let Err(e) = event.send(CmdCommand::RequestTask).await {
                                    eprintln!("Error fail to send command to backend! {e:?}");
                                }
                                break;
                            },
                        }
                    }
                    Err(e) => {
                        // This means there's something wrong with this task?
                        todo!("Please handle these errors: {e:?}");
                        // match &event.send(CmdCommand::RequestTask).await {
                        //     Ok(_) => {
                        //         sleep(Duration::from_secs(5u64)).await;
                        //     }
                        //     Err(e) => {
                        //         eprintln!("Fail to send command to network! {e:?}");
                        //     }
                        // }
                    }
                };
            }
        });

        // run cli mode in loop
        // let service_handler = 
        loop {
            select! {
                net_event = event_receiver.recv() => match net_event {
                    Some(event) => {
                        &self.handle_net_event(&mut client, event).await;
                        ()
                    },
                    None => return Err(NetworkError::Invalid),
                },
                msg = command.recv() => match msg {
                    Some(cmd) => {
                        &self.handle_command(&mut client, cmd).await;
                        ()
                    },
                    _ => (),
                }
            }
        }
    }
}
