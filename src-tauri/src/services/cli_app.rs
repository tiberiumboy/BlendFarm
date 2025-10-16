/*
Have a look into TUI for CLI status display window to show user entertainment on screen
https://docs.rs/tui/latest/tui/

Feature request:
    - See how we can treat this application process as service mode so that it can be initialize and start on machine reboot?
    - receive command to properly reboot computer when possible?
*/
use super::blend_farm::BlendFarm;
use crate::network::message::{self, Event, NetworkError, NodeEvent};
use crate::network::provider_rule::ProviderRule;
use crate::{
    domains::{job_store::JobError, task_store::TaskStore},
    models::{
        job::{Job, JobEvent},
        project_file::ProjectFile,
        server_setting::ServerSetting,
        task::Task,
    },
    network::controller::Controller,
};
use blender::blender::{Manager as BlenderManager, ManagerError};
use blender::models::event::BlenderEvent;
use libp2p::{Multiaddr, PeerId};
use std::time::Duration;
use std::{path::PathBuf, str::FromStr, sync::Arc};
use thiserror::Error;
use tokio::spawn;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::time::sleep;
use tokio::{select, sync::RwLock};
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
    task_store: Arc<RwLock<dyn TaskStore + Send + Sync + 'static>>,

    // config
    settings: ServerSetting,

    // The idea behind this is to let the network manager aware that the client side of the app is busy working on current task.
    // it would be nice to receive information and notification about this current client status somehow.
    // Could I use PhantomData to hold Task Object type?
    host: Option<(PeerId, Multiaddr)>, // isntead of this, we should hold task_handler. That way, we can abort it when we receive the invocation to do so.
}

impl CliApp {
    // we could simplify this design by just asking for the database info?
    pub fn new(task_store: Arc<RwLock<dyn TaskStore + Send + Sync + 'static>>) -> Self {
        let manager = BlenderManager::load();
        Self {
            settings: ServerSetting::load(),
            manager,
            task_store,
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
            let file_name = job.get_file_name_expected();

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

    // TODO: Rewrite this to meet Single responsibility principle.
    // How do I abort the job? -> That's the neat part! You don't! Delete the job+task entry from the database, and notify client to halt if running deleted jobs.
    /// Invokes the render job. The task needs to be mutable for frame deque.
    async fn render_task(
        &mut self,
        client: &mut Controller,
        task: &mut Task,
        sender: &mut Sender<BlenderEvent>,
    ) -> Result<(), CliError> {
        // for now, let's skip this part and continue on. We don't have DHT setup, but I want to make sure cli does actually render once we get the file share situation straighten out.
        // TODO: Find a way to get the file share working across network.
        // let project_file = self.validate_project_file(client, &task).await?;

        let job = AsRef::<Job>::as_ref(&task);
        let project_file = AsRef::<ProjectFile>::as_ref(&job);
        let version = job.as_ref();

        /*
        this script below was our internal implementation of handling DHT fallback mode
        save this for future feature updates
        let blender = match self.manager.have_blender(version) {
            Some(blend) => blend,
            None => {
                // when I do not have task blender version installed - two things will happen here before an error is thrown
                // First, check our internal DHT services to see if any other client on the network have matching version - then fetch it. Install after completion
                // Secondly, download the file online.
                // If we reach here - it is because no other node have matching version, and unable to connect to download url (Internet connectivity most likely).
                // TODO: It would be nice to broadcast everyone else "Hey! I'm download this version, could you wait until I'm done to distribute?"
                let link_name = &self
                    .manager
                    .get_blender_link_by_version(version)
                    .expect(&format!(
                        "Invalid Blender version used. Not found anywhere! Version {:?}",
                        &version
                    ))
                    .name;
                let destination = self.manager.get_install_path();

                // should also use this to send CmdCommands for network stuff.
                let latest = client.get_file_from_peers(&link_name, destination).await;

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
            }
        };
        */

        let blender = match self.manager.fetch_blender(version) {
            Ok(blender) => blender,
            Err(e) => {
                return Err(CliError::ManagerError(e));
            }
        };

        let id = AsRef::<Uuid>::as_ref(&task);
        let output = self
            .verify_and_check_render_output_path(id)
            .await
            .map_err(|e| CliError::Io(e))?;

        // run the job!
        // TODO: is there a better way to get around clone?
        match task
            .clone()
            .run(project_file.to_path_buf(), output, &blender)
            .await
        {
            Ok(rx) => loop {
                match rx.recv() {
                    Ok(status) => {
                        // SHould look into a better way to write this so that we can handle loop better for blender process....
                        // Somehow, receiver was closed?
                        match &status {
                            BlenderEvent::Completed { .. } => {
                                sender
                                    .send(status)
                                    .await
                                    .expect("Channel should not be closed");
                                // make sure to break out of this loop!
                                break;
                            }
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

                        // not sure if I still need this? 8/29/25
                        // let node_status = NodeEvent::BlenderStatus(status);
                        // client.send_node_status(node_status).await;
                    }
                    Err(e) => {
                        let event = BlenderEvent::Error(e.to_string());
                        sender.send(event).await.expect("Channel should be closed");
                        break;
                    }
                }
            },
            Err(e) => {
                let err = JobError::TaskError(e);
                client.send_job_event(JobEvent::Error(err)).await;
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
                {
                    let db = self.task_store.write().await;
                    // Need to make sure no other node work the same job here.
                    if let Err(e) = db.add_task(task.clone()).await {
                        println!("Unable to add task! {e:?}");
                    }
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
                let db = self.task_store.write().await;
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
                        eprintln!("Successfully rendered task!");
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
    async fn run(
        mut self,
        mut client: Controller,
        mut event_receiver: Receiver<Event>,
    ) -> Result<(), NetworkError> {
        // I need to find a way to safely notify the background to stop in case the job was deleted from host machine.
        // we will have one thread to process blender and queue, but I must have access to database.

        let (event, mut command) = mpsc::channel(32);

        // TODO: move this inside on discovery call
        // let cmd = CmdCommand::RequestTask;
        // event.send(cmd).await.expect("Should not be free?");

        let taskdb = self.task_store.clone();

        // background thread to handle blender invocation
        spawn(async move {
            loop {
                let db = taskdb.write().await;

                match db.poll_task().await {
                    Ok(result) => {
                        match result {
                            Some(task) => {
                                // why did this method get invoked twice?
                                println!("Begin some task!");
                                let (sender, mut receiver) = mpsc::channel(32);
                                let cmd = CmdCommand::Render(task.item, sender);
                                if let Err(e) = event.send(cmd).await {
                                    eprintln!("Fail to send backend service render request! {e:?}");
                                }

                                loop {
                                    select! {
                                        event = receiver.recv() => match event {
                                            Some(event) => {
                                                match event {
                                                    BlenderEvent::Log(log) => println!("{log}"),
                                                    BlenderEvent::Warning(warn) => println!("{warn}"),
                                                    BlenderEvent::Rendering { current, total } => println!("Rendering {current} out of {total}"),
                                                    BlenderEvent::Completed { result, .. } => {
                                                        println!("Image completed! {result:?}")
                                                    },
                                                    // receiving unhandled event for getting blender version and commit hash value?
                                                    BlenderEvent::Unhandled(e) => {
                                                        // Blender 4.3.2 (hash 32f5fdce0a0a built 2024-12-17 02:14:25)
                                                        eprintln!("{e:?}");
                                                    },
                                                    BlenderEvent::Exit => {
                                                        println!("Blender exit! This task should be completed?");
                                                        if let Err(e) = db.delete_task(&task.id).await {
                                                            // if the task doesn't exist
                                                            eprintln!(
                                                                "Fail to delete task entry from database! {e:?}"
                                                            );
                                                        }
                                                        break;
                                                    },
                                                    BlenderEvent::Error(e) => {
                                                        eprintln!("Received Blender Error: {e:?}");
                                                        break
                                                    },
                                                }
                                            },
                                            None => {
                                                eprintln!("Received None from Blender loop! Breaking");
                                                break
                                            }
                                        }
                                    }
                                }
                            }
                            None => match event.send(CmdCommand::RequestTask).await {
                                Ok(_) => {
                                    sleep(Duration::from_secs(5u64)).await;
                                }
                                Err(e) => {
                                    eprintln!("Error fail to send command to backend! {e:?}");
                                    sleep(Duration::from_secs(5u64)).await;
                                }
                            },
                        }
                    }
                    Err(e) => {
                        eprintln!("Issue polling task from db: {e:?}");
                        match event.send(CmdCommand::RequestTask).await {
                            Ok(_) => {
                                sleep(Duration::from_secs(5u64)).await;
                            }
                            Err(e) => {
                                eprintln!("Fail to send command to network! {e:?}");
                            }
                        }
                    }
                };
            }
        });

        // run cli mode in loop
        loop {
            select! {
                net_event = event_receiver.recv() => match net_event {
                    Some(event) => self.handle_net_event(&mut client, event).await,
                    None => return Err(NetworkError::Invalid),
                },
                msg = command.recv() => match msg {
                    Some(cmd) => self.handle_command(&mut client, cmd).await,
                    _ => (),
                }
            }
        }
    }
}
