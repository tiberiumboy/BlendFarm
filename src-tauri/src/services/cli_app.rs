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
    domains::job_store::JobStore,
    models::{
        job::{Job, JobEvent},
        message::{NetEvent, NetworkError},
        network::{NetworkController, JOB},
    },
};
use async_trait::async_trait;
use blender::blender::Manager as BlenderManager;
use blender::models::status::Status;
use tokio::{
    select,
    sync::{mpsc::Receiver, RwLock},
};

pub struct CliApp {
    manager: BlenderManager,
    job_store: Arc<RwLock<(dyn JobStore + Send + Sync + 'static)>>,
}

impl CliApp {
    pub fn new(job_store: Arc<RwLock<(dyn JobStore + Send + Sync + 'static)>>) -> Self {
        let manager = BlenderManager::load();
        Self { manager, job_store }
    }
}

impl CliApp {
    // TODO: May have to refactor this to take consideration of Job Storage
    async fn render_job(&mut self, client: &mut NetworkController, job: Job) {
        let status = format!("Receive render job [{}]", job.as_ref());
        client.send_status(status).await;

        let file_name = job.get_file_name().unwrap().to_string();
        let id = job.as_ref().clone();
        // create a path link where we think the file should be
        let blend_dir = client.settings.blend_dir.join(id.to_string());
        if let Err(e) = async_std::fs::create_dir_all(&blend_dir).await {
            eprintln!("Error creating blend directory! {e:?}");
        }
        // assume project file is located inside this directory.
        let project_file = blend_dir.join(&file_name); // append the file name here instead.

        client
            .send_status(format!("Checking for project file {:?}", &project_file))
            .await;
        let mut job = job.set_project_path(project_file.clone());

        // Fetch the project from peer if we don't have it.
        if !project_file.exists() {
            println!(
                "Project file do not exist, asking to download from host: {:?}",
                &file_name
            );
            // TODO: To receive the path or not to modify existing project_file value? I expect both would have the same value?
            match client.get_file_from_peers(&file_name, &blend_dir).await {
                Ok(path) => println!("File successfully download from peers! path: {path:?}"),
                Err(e) => match e {
                    NetworkError::UnableToListen(_) => todo!(),
                    NetworkError::NotConnected => todo!(),
                    NetworkError::SendError(_) => {}
                    NetworkError::NoPeerProviderFound => {
                        client
                            .send_status("No peer provider founkd on the network?".to_owned())
                            .await
                    }
                    NetworkError::UnableToSave(e) => {
                        client
                            .send_status(format!("Fail to save file to disk: {e}"))
                            .await
                    }
                    _ => println!("Unhandle error received {e:?}"), // shouldn't be covered?
                },
            }
        }

        // here we'll ask if we have blender installed before usage
        let blender = self
            .manager
            .fetch_blender(job.get_version())
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
        match job.run(output, &blender).await {
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
                        Status::Completed { frame, result, .. } => {
                            let file_name = format!(
                                "{}_{}",
                                id.to_string(),
                                result.file_name().unwrap().to_str().unwrap().to_string()
                            );
                            let event = JobEvent::ImageCompleted {
                                job_id: id,
                                frame,
                                file_name: file_name.clone(),
                            };
                            client.start_providing(file_name, result).await;
                            // here how do I get the job's requestor?
                            client.send_job_message(requestor, event).await;
                        }
                        Status::Exit => {
                            client.send_job_message(JobEvent::JobComplete).await;
                            break;
                        }
                    };
                }
            },
            Err(e) => {
                client.send_job_message(JobEvent::Error(e)).await;
            }
        };
    }

    async fn handle_message(&mut self, client: &mut NetworkController, event: NetEvent) {
        match event {
            NetEvent::OnConnected => client.share_computer_info().await,
            NetEvent::NodeDiscovered(..) => {}  // Ignored
            NetEvent::NodeDisconnected(_) => {} // ignored
            NetEvent::JobUpdate(job_event) => match job_event {
                JobEvent::Render(job) => {
                    {
                        let mut db = self.job_store.write().await;
                        if let Err(e) = db.add_job(job.clone()).await {
                            eprintln!("Fail to save job to database! {e:?}");
                        } else {
                            println!("Added job to database!");
                        }
                    };
                    self.render_job(client, job).await;
                }
                JobEvent::ImageCompleted { .. } => {} // ignored since we do not want to capture image?
                // For future impl. we can take advantage about how we can allieve existing job load. E.g. if I'm still rendering 50%, try to send this node the remaining parts?
                JobEvent::JobComplete => {} // Ignored, we're treated as a client node, waiting for new job request.
                JobEvent::Remove(id) => {
                    let mut db = self.job_store.write().await;
                    if let Err(e) = db.delete_job(id).await {
                        eprintln!("Fail to remove job from database! {e:?}");
                    } else {
                        println!("Successfully remove job from database!");
                    }
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

#[async_trait]
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
        // client.subscribe_to_topic(system_info).await;
        client.subscribe_to_topic(JOB.to_string()).await;

        // let current_job: Option<Job> = None;
        loop {
            select! {
                // here we can insert job_db here to receive event invocation from Tauri_app
                Some(event) = event_receiver.recv() => self.handle_message(&mut client, event).await,

                // here we can fetch the next job available and see about what we can do to render the scene?
                // Some(job) = current_job.take() => {
                //     let db = self.job_store.write().await;

                //     current_job = self.
                // }
                // how do I poll the machine specs in certain intervals?
                // Some(event) = self.machine.
            }
        }
        // if somehow we were able to get out of the loop, we would best send a shutdown notice here.
        // client.shutdown().await;
    }
}
