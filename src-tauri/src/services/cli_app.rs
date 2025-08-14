use async_std::task::sleep;
use std::{path::PathBuf, sync::Arc, time::Duration};
/*
Have a look into TUI for CLI status display window to show user entertainment on screen
https://docs.rs/tui/latest/tui/

Feature request:
    - See how we can treat this application process as service mode so that it can be initialize and start on machine reboot?
    - receive command to properly reboot computer when possible?
*/
use super::blend_farm::BlendFarm;
use crate::{
    domains::{job_store::JobError, task_store::TaskStore},
    models::{
        job::JobEvent,
        message::{self, Event, NetworkError},
        network::{NetworkController, NodeEvent},
        server_setting::ServerSetting,
        task::Task,
    },
};
use blender::models::event::BlenderEvent;
use blender::{
    blender::{Blender, Manager as BlenderManager},
    models::download_link::DownloadLink,
};
use futures::{
    SinkExt, StreamExt,
    channel::mpsc::{self, Receiver, Sender},
};
use std::path::Path;
use thiserror::Error;
use tokio::{select, spawn, sync::RwLock};
use uuid::Uuid;

enum CmdCommand {
    Render(Task, Sender<BlenderEvent>),
    RequestTask, // calls to host for more task.
}

#[derive(Debug, Error)]
enum CliError {
    #[error("Encounter an network error! \n{0:}")]
    NetworkError(#[from] message::NetworkError),
    #[error("Encounter an IO error! \n{0}")]
    Io(#[from] async_std::io::Error),
}

pub struct CliApp {
    manager: BlenderManager,
    task_store: Arc<RwLock<dyn TaskStore + Send + Sync + 'static>>,
    settings: ServerSetting,
    // The idea behind this is to let the network manager aware that the client side of the app is busy working on current task.
    // it would be nice to receive information and notification about this current client status somehow.
    // Could I use PhantomData to hold Task Object type?
    #[allow(dead_code)]
    task_handle: Option<Task>, // isntead of this, we should hold task_handler. That way, we can abort it when we receive the invocation to do so.
}

impl CliApp {
    pub fn new(task_store: Arc<RwLock<dyn TaskStore + Send + Sync + 'static>>) -> Self {
        let manager = BlenderManager::load();
        Self {
            settings: ServerSetting::load(),
            manager,
            task_store,
            task_handle: None, // no task assigned yet
        }
    }
}

impl CliApp {
    async fn check_project_file(
        client: &mut NetworkController,
        task: &Task,
        search_directory: &Path,
    ) -> Result<PathBuf, CliError> {
        let job = task.get_job();
        let file_name = job.get_file_name_expected();

        // TODO: To receive the path or not to modify existing project_file value? I expect both would have the same value?
        client
            .get_file_from_peers(&file_name, search_directory)
            .await
            .map_err(CliError::NetworkError)
    }

    // This function will ensure the directory will exist, and return the path to that given directory.
    // It will remain valid unless directory or parent above is removed during runtime.
    async fn generate_temp_project_task_directory(
        settings: &ServerSetting,
        task: &Task,
        id: &str,
    ) -> Result<PathBuf, async_std::io::Error> {
        // create a path link where we think the file should be
        let job = task.get_job();
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

    async fn validate_project_file(
        &self,
        client: &mut NetworkController,
        task: &Task,
    ) -> Result<PathBuf, CliError> {
        let id = task.get_id();
        let project_file_path =
            CliApp::generate_temp_project_task_directory(&self.settings, &task, &id.to_string())
                .await
                .expect("Should have permission!");

        // assume project file is located inside this directory.
        println!("Checking for {:?}", &project_file_path);

        let job = task.get_job();
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
            CliApp::check_project_file(client, task, search_directory).await?;
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
        client: &mut NetworkController,
        task: &mut Task,
        sender: &mut Sender<BlenderEvent>,
    ) -> Result<(), CliError> {
        let project_file = self.validate_project_file(client, &task).await?;

        println!("Ok we expect to have the project file available, now let's check for Blender");

        // am I'm introducing multiple behaviour in this single function?
        let job = task.get_job();
        let version = &job.get_version();
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

        let output = self
            .verify_and_check_render_output_path(task.get_id())
            .await
            .map_err(|e| CliError::Io(e))?;

        // run the job!
        // TODO: is there a better way to get around clone?
        match task.clone().run(project_file, output, &blender).await {
            Ok(rx) => loop {
                if let Ok(status) = rx.recv() {
                    sender
                        .send(status)
                        .await
                        .expect("Channel should not be closed");
                    // not sure if I still need this?
                    // let node_status = NodeEvent::BlenderStatus(status);
                    // client.send_node_status(node_status).await;
                }
            },
            Err(e) => {
                let (sender, mut receiver) = mpsc::channel(1);
                let err = JobError::TaskError(e);
                client.send_job_event(JobEvent::Error(err), sender).await;

                if let Err(e) = receiver.select_next_some().await {
                    eprintln!("fail to send job! {e:?}");
                    sleep(Duration::from_secs(5u64)).await;
                }
            }
        };

        Ok(())
    }

    async fn handle_job_update(&mut self, event: JobEvent) {
        match event {
            // on render task received, we should store this in the database.
            JobEvent::Render(task) => {
                println!("Received new Render Task! Added to Queue!!");

                let db = self.task_store.write().await;
                if let Err(e) = db.add_task(task).await {
                    println!("Unable to add task! {e:?}");
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
    async fn handle_net_event(&mut self, client: &mut NetworkController, event: Event) {
        match event {
            Event::JobUpdate(job_event) => self.handle_job_update(job_event).await,
            Event::InboundRequest { request, channel } => {
                self.handle_inbound_request(client, request, channel).await
            }
            Event::NodeStatus(event) => {
                match event {
                    NodeEvent::Connected(peer_id, spec) => {
                        // peer connected with specs.
                        println!("Peer connecte with specs provided : {peer_id:?}\n{spec:?}");
                        // println!("Requesting task");
                        // let event = JobEvent::RequestTask;
                        // client.send_job_event(event).await;
                    }
                    NodeEvent::Disconnected { peer_id, reason } => match reason {
                        Some(err) => {
                            println!("Peer Disconnected with reason [{peer_id:?}] {err}");
                        }
                        None => println!("Peer Disconnected without reason! [{peer_id:?}]"),
                    },
                    NodeEvent::BlenderStatus(blender_event) => {
                        println!("[Blender Status] {blender_event:?}");
                    }
                }
            }
            _ => println!("[CLI] Unhandled event from network: {event:?}"),
        }
    }

    async fn handle_command(&mut self, client: &mut NetworkController, cmd: CmdCommand) {
        match cmd {
            CmdCommand::Render(mut task, mut sender) => {
                // TODO: We should find a way to mark this node currently busy so we should unsubscribe any pending new jobs if possible?
                // mutate this struct to skip listening for any new jobs.
                // proceed to render the task.
                if let Err(e) = self.render_task(client, &mut task, &mut sender).await {
                    let (sender, mut receiver) = mpsc::channel(1);
                    let event = JobEvent::Failed(e.to_string());
                    client.send_job_event(event, sender).await;

                    if let Err(e) = receiver.select_next_some().await {
                        eprintln!("Fail top send job event! {e:?}");
                        sleep(Duration::from_secs(5u64)).await;
                    }
                }
            }

            CmdCommand::RequestTask => {
                // or at least have this node look into job history and start working on jobs that are not completed yet.
                let (sender, mut receiver) = mpsc::channel(1);
                let event = JobEvent::RequestTask;
                client.send_job_event(event, sender).await;

                if let Err(e) = receiver.select_next_some().await {
                    eprintln!("Fail to send job event! {e:?}");
                    sleep(Duration::from_secs(5u64)).await;
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl BlendFarm for CliApp {
    async fn run(
        mut self,
        mut client: NetworkController,
        mut event_receiver: Receiver<Event>,
    ) -> Result<(), NetworkError> {
        // I need to find a way to safely notify the background to stop in case the job was deleted from host machine.
        // we will have one thread to process blender and queue, but I must have access to database.
        let taskdb = self.task_store.clone();
        let (mut event, mut command) = mpsc::channel(32);

        // background thread to handle blender invocation
        spawn(async move {
            loop {
                // get the first task if exist.
                let db = taskdb.write().await;

                match db.poll_task().await {
                    Ok(result) => {
                        match result {
                            Some(task) => {
                                println!("Got task to do! {task:?}");
                                let (sender, mut receiver) = mpsc::channel(32);
                                let cmd = CmdCommand::Render(task.item, sender);
                                if let Err(e) = event.send(cmd).await {
                                    eprintln!("Fail to send backend service render request! {e:?}");
                                }

                                loop {
                                    select! {
                                        event = receiver.select_next_some() => {
                                            match event {
                                                BlenderEvent::Log(log) => println!("{log}"),
                                                BlenderEvent::Warning(warn) => println!("{warn}"),
                                                BlenderEvent::Rendering { current, total } => println!("Rendering {current} out of {total}"),
                                                BlenderEvent::Completed { result, .. } => println!("Image completed! {result:?}"),
                                                BlenderEvent::Unhandled(e) => {
                                                    eprintln!("Unahandle blender event received! {e:?}");
                                                    break;
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
                                                BlenderEvent::Error(_) => break,
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
                event = event_receiver.select_next_some() => self.handle_net_event(&mut client, event).await,
                msg = command.select_next_some() => self.handle_command(&mut client, msg).await,
            }
        }
    }
}
