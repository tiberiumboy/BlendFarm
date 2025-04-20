use std::{path::PathBuf, sync::Arc, thread::sleep, time::Duration};

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
        message::{NetEvent, NetworkError},
        network::{NetworkController, JOB},
        server_setting::ServerSetting,
        task::Task,
    },
};
use blender::models::status::Status;
use blender::{
    blender::{Blender, Manager as BlenderManager},
    models::download_link::DownloadLink,
};
use thiserror::Error;
use tokio::{
    select, spawn,
    sync::{
        mpsc::{self, Receiver},
        RwLock,
    },
};

enum CmdCommand {
    Render(Task),
}

#[derive(Debug, Error)]
enum CliError {
    #[error("Received Network Issue: {0}")]
    NetworkError(String),
    #[error("Unknown error received: {0}")]
    Unknown(String),
    #[error("Unable to listen - Connection rejected?")]
    ConnectionRejected,
    #[error("Not connected")]
    NotConnected,
}

pub struct CliApp {
    manager: BlenderManager,
    task_store: Arc<RwLock<(dyn TaskStore + Send + Sync + 'static)>>,
    settings: ServerSetting,
    // Hmm not sure if I need this but we'll see!
    // task_handle: Option<JoinHandle<()>>, // isntead of this, we should hold task_handler. That way, we can abort it when we receive the invocation to do so.
}

impl CliApp {
    pub fn new(task_store: Arc<RwLock<(dyn TaskStore + Send + Sync + 'static)>>) -> Self {
        let manager = BlenderManager::load();
        Self {
            settings: ServerSetting::load(),
            manager,
            task_store,
        }
    }
}

impl CliApp {
    async fn check_project_file(
        client: &mut NetworkController,
        task: &mut Task,
        search_directory: &PathBuf,
    ) -> Result<PathBuf, NetworkError> {
        let file_name = task.blend_file_name.to_str().unwrap();

        println!("Calling network for project file {file_name}");

        // TODO: To receive the path or not to modify existing project_file value? I expect both would have the same value?
        client
            .get_file_from_peers(&file_name, search_directory)
            .await
    }

    // TODO: Rewrite this to meet Single responsibility principle.
    // How do I abort the job? -> That's the neat part! You don't! Delete the job+task entry from the database, and notify client to halt if running deleted jobs.
    /// Invokes the render job. The task needs to be mutable for frame deque.
    async fn render_task(
        &mut self,
        client: &mut NetworkController,
        task: &mut Task,
    ) -> Result<(), CliError> {
        let id = task.job_id;

        // create a path link where we think the file should be
        let blend_dir = self.settings.blend_dir.join(id.to_string());
        if let Err(e) = async_std::fs::create_dir_all(&blend_dir).await {
            eprintln!("Error creating blend directory! {e:?}");
        }

        // assume project file is located inside this directory.
        let project_file = blend_dir.join(&task.blend_file_name); // append the file name here instead.

        println!("Checking for {:?}", &project_file);

        // Fetch the project from peer if we don't have it.
        if !project_file.exists() {
            println!(
                "Project file do not exist, asking to download from DHT: {:?}",
                &task.blend_file_name
            );

            // so I need to figure out something about this...
            // TODO - find a way to break out of this if we can't fetch the project file.
            if let Err(e) = CliApp::check_project_file(client, task, &blend_dir).await {
                // let the host know hey we can't do this job because reason
                eprintln!("Fail to get project file: {e:?}");
                return Err(CliError::Unknown(e.to_string()));
            }
        }

        println!("Ok we have project file, now check for Blender");

        // am I'm introducing multiple behaviour in this single function?
        let blender = match self.manager.have_blender(&task.blender_version) {
            Some(blend) => blend,
            None => {
                // when I do not have task blender version installed - two things will happen here before an error is thrown
                // First, check our internal DHT services to see if any other client on the network have matching version - then fetch it. Install after completion
                // Secondly, download the file online.
                // If we reach here - it is because no other node have matching version, and unable to connect to download url (Internet connectivity most likely).
                // TODO: It would be nice to broadcast everyone else "Hey! I'm download this version, could you wait until I'm done to distribute?"
                let v = &task.blender_version;
                let link_name = &self
                    .manager
                    .home
                    .get_version(v.major, v.minor)
                    .expect(&format!(
                        "Invalid Blender version used. Not found anywhere! Version {:?}",
                        &task.blender_version
                    ))
                    .name;
                // should also use this to send CmdCommands for network stuff.
                let latest = client.get_file_from_peers(&link_name, &blend_dir).await;

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
                        println!("No client on network is advertising target blender installation! {e:?}");
                        &self
                            .manager
                            .fetch_blender(&task.blender_version)
                            .expect("Fail to download blender")
                    }
                }
            }
        };
        //     }
        // }

        // create a output destination for the render image
        let output = self.settings.render_dir.join(id.to_string());
        if let Err(e) = async_std::fs::create_dir_all(&output).await {
            eprintln!("Error creating render directory: {e:?}");
        }

        // run the job!
        // TODO: is there a better way to get around clone?
        match task.clone().run(project_file, output, &blender).await {
            Ok(rx) => loop {
                if let Ok(status) = rx.recv() {
                    match status {
                        Status::Idle => client.send_status("[Idle]".to_owned()).await,
                        Status::Running { status } => {
                            client.send_status(format!("[Running] {status}")).await
                        }
                        Status::Log { status } => {
                            client.send_status(format!("[Log] {status}")).await
                        }
                        Status::Warning { message } => {
                            client.send_status(format!("[Warning] {message}")).await
                        }
                        Status::Error(blender_error) => {
                            client.send_status(format!("[ERR] {blender_error:?}")).await
                        }

                        Status::Completed { frame, result } => {
                            let file_name = result.file_name().unwrap().to_string_lossy();
                            let file_name = format!("/{}/{}", task.job_id, file_name);
                            let event = JobEvent::ImageCompleted {
                                job_id: task.job_id,
                                frame,
                                file_name: file_name.clone(),
                            };

                            client.start_providing(file_name, result).await;
                            client.send_job_message(&task.requestor, event).await;
                        }
                        Status::Exit => {
                            // hmm is this technically job complete?
                            // Check and see if we have any queue pending, otherwise ask hosts around for available job queue.
                            // sender.send(CmdCommand::TaskComplete(task.into())).await;
                            println!("Task complete, breaking loop!");
                            break;
                        }
                    };
                }
            },
            Err(e) => {
                let err = JobError::TaskError(e);
                client
                    .send_job_message(&task.requestor, JobEvent::Error(err))
                    .await;
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
            JobEvent::JobComplete => {} // Ignored, we're treated as a client node, waiting for new job request.
            // Remove all task with matching job id.
            JobEvent::Remove(job_id) => {
                let db = self.task_store.write().await;
                if let Err(e) = db.delete_job_task(&job_id).await {
                    eprintln!("Unable to remove all task with matching job id! {e:?}");
                }
            }
            _ => println!("Unhandle Job Event: {event:?}"),
        }
    }

    async fn handle_net_event(&mut self, client: &mut NetworkController, event: NetEvent) {
        match event {
            NetEvent::OnConnected(peer_id) => client.share_computer_info(peer_id).await,

            NetEvent::JobUpdate(job_event) => self.handle_job_update(job_event).await,
            // maybe move this inside Network code? Seems repeative in both cli and Tauri side of application here.
            NetEvent::InboundRequest { request, channel } => {
                if let Some(path) = client.file_service.providing_files.get(&request) {
                    println!("Sending file {path:?}");

                    // this responded back to the network controller? Why?
                    client
                        .respond_file(std::fs::read(path).unwrap(), channel)
                        .await;
                }
            }
            NetEvent::NodeDiscovered(..) => {}  // Ignored
            NetEvent::NodeDisconnected(_) => {} // ignored
            _ => println!("[CLI] Unhandled event from network: {event:?}"),
        }
    }

    async fn handle_command(&mut self, client: &mut NetworkController, cmd: CmdCommand) {
        match cmd {
            CmdCommand::Render(mut task) => {
                if let Err(e) = self.render_task(client, &mut task).await {
                    client
                        .send_job_message(&task.requestor, JobEvent::Failed(e.to_string()))
                        .await
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
        mut event_receiver: Receiver<NetEvent>,
    ) -> Result<(), NetworkError> {
        // Future Impl. Make this machine available to other peers that share the same operating system and arch
        // - so that we can distribute blender across network rather than download blender per each peers.
        // let system = self.machine.system_info();
        // let system_info = format!("blendfarm/{}{}", consts::OS, &system.processor.brand);
        // TODO: Figure out why I need the JOB subscriber?
        client.subscribe_to_topic(JOB.to_string()).await;
        client.subscribe_to_topic(client.hostname.clone()).await;

        // could we just run a background thread here to handle task job?
        // I need to find a way to safely notify the background to stop in case the job was deleted from host machine.

        // we will have one thread to process blender and queue, but I must have access to database.
        let taskdb = self.task_store.clone();
        let (event, mut command) = mpsc::channel(32);

        // background thread to handle blender invocation
        spawn(async move {
            loop {
                // get the first task if exist.
                // I don't want to spam the database for pending task?
                let db = taskdb.write().await;
                // so why can't I get this to work?
                if let Ok(task_dto) = db.poll_task().await {
                    if let Err(e) = db.delete_task(&task_dto.id).await {
                        eprintln!("Fail to delete task entry from database! {task_dto:?} \n{e:?}");
                    }

                    let task = task_dto.item.clone();

                    if let Err(e) = event.send(CmdCommand::Render(task)).await {
                        eprintln!("Fail to send render command! {e:?}");
                    }
                } else {
                    println!("No task found! Sleeping...");
                    sleep(Duration::from_secs(2u64));
                }
            }
        });

        loop {
            select! {
                Some(event) = event_receiver.recv() => self.handle_net_event(&mut client, event).await,
                Some(msg) = command.recv() => self.handle_command(&mut client, msg).await,
            }
        }
    }
}
