use std::sync::Arc;

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
        task::Task,
    },
};
use blender::blender::Manager as BlenderManager;
use blender::models::status::Status;
use tokio::{
    select,
    sync::{mpsc::Receiver, RwLock},
};

pub struct CliApp {
    manager: BlenderManager,
    task_store: Arc<RwLock<(dyn TaskStore + Send + Sync + 'static)>>,
    // Hmm not sure if I need this but we'll see!
    // task_handle: Option<JoinHandle<()>>, // isntead of this, we should hold task_handler. That way, we can abort it when we receive the invocation to do so.
}

impl CliApp {
    pub fn new(task_store: Arc<RwLock<(dyn TaskStore + Send + Sync + 'static)>>) -> Self {
        let manager = BlenderManager::load();
        Self {
            manager,
            task_store,
            // task_handle: None,
        }
    }
}

impl CliApp {
    // TODO: May have to refactor this to take consideration of Job Storage
    // How do I abort the job?
    // Invokes the render job. The task needs to be mutable for frame deque.
    async fn render_task(
        &mut self,
        client: &mut NetworkController,
        hostname: &str,
        task: &mut Task,
    ) {
        let status = format!("Receive task from peer [{:?}]", task);
        client.send_status(status).await;
        let id = task.job_id;

        // create a path link where we think the file should be
        let blend_dir = client.settings.blend_dir.join(id.to_string());
        if let Err(e) = async_std::fs::create_dir_all(&blend_dir).await {
            eprintln!("Error creating blend directory! {e:?}");
        }

        // assume project file is located inside this directory.
        let project_file = blend_dir.join(&task.blend_file_name); // append the file name here instead.

        client
            .send_status(format!("Checking for project file {:?}", &project_file))
            .await;

        // Fetch the project from peer if we don't have it.
        if !project_file.exists() {
            println!(
                "Project file do not exist, asking to download from host: {:?}",
                &task.blend_file_name
            );

            let file_name = task.blend_file_name.to_str().unwrap();
            // TODO: To receive the path or not to modify existing project_file value? I expect both would have the same value?
            match client.get_file_from_peers(&file_name, &blend_dir).await {
                Ok(path) => println!("File successfully download from peers! path: {path:?}"),
                Err(e) => match e {
                    NetworkError::UnableToListen(_) => todo!(),
                    NetworkError::NotConnected => todo!(),
                    NetworkError::SendError(_) => {}
                    NetworkError::NoPeerProviderFound => {
                        // I was timed out here?
                        client
                            .send_status("No peer provider found on the network?".to_owned())
                            .await
                    }
                    NetworkError::UnableToSave(e) => {
                        client
                            .send_status(format!("Fail to save file to disk: {e}"))
                            .await
                    }
                    NetworkError::Timeout => {
                        // somehow we lost connection, try to establish connection again?
                        // client.dial(request_id, client.public_addr).await;
                        dbg!("Timed out?");
                    }
                    _ => println!("Unhandle error received {e:?}"), // shouldn't be covered?
                },
            }
        }

        // here we'll ask if we have blender installed before usage
        let blender = self
            .manager
            .fetch_blender(&task.blender_version)
            .expect("Fail to download blender");

        // TODO: Call other network on specific topics to see if there's a version available.
        // match manager.have_blender(job.as_ref()) {
        //     Some(exe) => exe.clone(),
        //     None => {
        //         // try to fetch from other peers with matching os / arch.
        //         // question is, how do I make them publicly available with the right blender version? or do I just find it by the executable name instead?
        //     }
        // }

        // create a output destination for the render image
        let output = client.settings.render_dir.join(id.to_string());
        if let Err(e) = async_std::fs::create_dir_all(&output).await {
            eprintln!("Error creating render directory: {e:?}");
        }

        // run the job!
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
                            let file_name = format!("/{}/{}", id, file_name);
                            let event = JobEvent::ImageCompleted {
                                job_id: id,
                                frame,
                                file_name: file_name.clone(),
                            };
                            // send message back
                            client.start_providing(file_name, result).await;
                            client.send_job_message(hostname, event).await;
                        }
                        Status::Exit => {
                            client
                                .send_job_message(hostname, JobEvent::JobComplete)
                                .await;
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
    }

    async fn handle_message(&mut self, client: &mut NetworkController, event: NetEvent) {
        match event {
            NetEvent::OnConnected(peer_id) => client.share_computer_info(peer_id).await,
            NetEvent::NodeDiscovered(..) => {}  // Ignored
            NetEvent::NodeDisconnected(_) => {} // ignored
            NetEvent::JobUpdate(hostname, job_event) => match job_event {
                // on render task received, we should store this in the database.
                JobEvent::Render(mut task) => {
                    // TODO: consider adding a poll/queue for all of the pending task to work on.
                    // This poll can be queued by other nodes to check if this node have any pending task to work on.
                    // This will help us balance our workstation priority flow.
                    // for now we'll try to get one job to focused on.
                    self.render_task(client, &hostname, &mut task).await
                }
                JobEvent::ImageCompleted { .. } => {} // ignored since we do not want to capture image?
                // For future impl. we can take advantage about how we can allieve existing job load. E.g. if I'm still rendering 50%, try to send this node the remaining parts?
                JobEvent::JobComplete => {} // Ignored, we're treated as a client node, waiting for new job request.
                // Remove what exactly? Task? Job?
                JobEvent::Remove(id) => {
                    let db = self.task_store.write().await;
                    let _ = db.delete_job_task(&id).await;
                    // let mut db = self.job_store.write().await;
                    // if let Err(e) = db.delete_job(id).await {
                    //     eprintln!("Fail to remove job from database! {e:?}");
                    // } else {
                    //     println!("Successfully remove job from database!");
                    // }
                }
                _ => println!("Unhandle Job Event: {job_event:?}"),
            },
            // maybe move this inside Network code? Seems repeative in both cli and Tauri side of application here.
            NetEvent::InboundRequest { request, channel } => {
                if let Some(path) = client.providing_files.get(&request) {
                    println!("Sending file {path:?}");
                    client
                        .respond_file(std::fs::read(path).unwrap(), channel)
                        .await;
                }
            }
            _ => println!("[CLI] Unhandled event from network: {event:?}"),
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

        loop {
            select! {
                // here we can insert job_db here to receive event invocation from Tauri_app
                Some(event) = event_receiver.recv() => self.handle_message(&mut client, event).await,
                // how do I poll database here?
                // how do I poll the machine specs in certain intervals for activity monitor reading?
            }
        }
    }
}
