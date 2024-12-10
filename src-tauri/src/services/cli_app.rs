/*
Have a look into TUI for CLI status display window to show user entertainment on screen
https://docs.rs/tui/latest/tui/
*/
use super::blend_farm::BlendFarm;
use blender::blender::Manager as BlenderManager;
use crate::models::{
    job::{Job, JobEvent},
    message::{NetEvent, NetworkError},
    network::{NetworkController, JOB},
};
use async_trait::async_trait;
use blender::models::status::Status;
// use machine_info::Machine;
use std::{collections::HashMap, /* env::consts, */ path::PathBuf};
use tokio::{select, sync::mpsc::Receiver};

pub struct CliApp {
    // machine: Machine,
    // job that this machine is busy working on.
    #[allow(dead_code)]
    active_job: Option<Job>,
    providing_files : HashMap<String, PathBuf>,
    manager: BlenderManager,
}

impl Default for CliApp {
    fn default() -> Self {
        Self {
            // machine: Machine::new(),
            active_job: Default::default(),
            providing_files: Default::default(),
            manager: BlenderManager::load(),
        }
    }
}

impl CliApp {
    async fn render_job(&mut self, controller: &mut NetworkController, job: Job) {
        let status = format!("Receive render job [{}]", job.as_ref());
        controller.send_status(status).await; 
        
        let output = controller.settings.render_dir.clone();
        let file_name = job.get_file_name().unwrap().to_string();
        
        // create a path link where we think the file should be?
        let blend_dir = controller.settings.blend_dir.clone(); 
        let project_file = blend_dir.join(&file_name); // append the file name here instead.
        controller.send_status(format!("Checking for project file {:?}", &project_file)).await;
        let mut job = job.set_project_path(project_file.clone());

        // Fetch the project from peer if we don't have it.
        if !project_file.exists() {
            println!("Project file do not exist, asking to download from host: {:?}", &file_name);    
            match controller.get_file_from_peers(&file_name, &blend_dir).await {
                Ok(_) => println!("File successfully download from peers!"),
                Err(e) => match e {
                    NetworkError::UnableToListen(_) => todo!(),
                    NetworkError::NotConnected => todo!(),
                    NetworkError::SendError(_) => {},
                    NetworkError::NoPeerProviderFound => {
                        controller.send_status("No peer provider founkd on the network?".to_owned()).await
                    },
                    NetworkError::UnableToSave(e) => {
                        controller.send_status(format!("Fail to save file to disk: {e}")).await
                    },
                    _ => println!("Unhandle error received {e:?}") // shouldn't be covered?
                }    
            }
        }

        // here we'll ask if we have blender installed before usage
        let blender = self.manager.fetch_blender(job.get_version()).expect("Fail to download blender");

        // TODO: Call other network on specific topics to see if there's a version available.
        // match manager.have_blender(job.as_ref()) {
        //     Some(exe) => exe.clone(),
        //     None => {
        //         // try to fetch from other peers with matching os / arch.
        //         // question is, how do I make them publicly available with the right blender version? or do I just find it by the executable name instead?

        //     }
        // }
        
        // run the job!
        match job.run(output, &blender).await {
            Ok(rx) => {
                loop {
                    if let Ok(status) = rx.recv() {
                        match status {
                            Status::Idle => controller.send_status("[Idle]".to_owned()).await,
                            Status::Running { status } => controller.send_status(format!("[Running] {status}")).await,
                            Status::Log { status } => controller.send_status(format!("[Log] {status}")).await,
                            Status::Warning { message } => controller.send_status(format!("[Warning] {message}")).await,
                            Status::Error(blender_error) => controller.send_status(format!("[ERR] {blender_error:?}")).await,
                            Status::Completed { frame, result, .. } => {
                                let file_name = result.file_name().unwrap().to_str().unwrap().to_string();
                                self.providing_files.insert(file_name.clone(), result);
                                let event = JobEvent::ImageCompleted { id: job.as_ref().clone(), frame, file_name: file_name.clone() };
                                controller.start_providing(file_name).await;
                                controller.send_job_message(event).await;
                            },
                            Status::Exit => {
                                controller.send_job_message(JobEvent::JobComplete).await;
                                break;
                            }
                        };
                    }
                }
            },
            Err(e) => {
                controller.send_job_message(JobEvent::Error(e)).await;
            }
        };
    }

    async fn handle_message(&mut self, controller: &mut NetworkController, event: NetEvent) {
        match event {
            NetEvent::OnConnected => controller.share_computer_info().await,
            NetEvent::NodeDiscovered(..) => { } // Ignored
            NetEvent::NodeDisconnected(_) => {} // ignored
            NetEvent::JobUpdate(job_event) => match job_event {
                JobEvent::Render(job) => self.render_job(controller, job).await,
                JobEvent::ImageCompleted { .. } => {} // ignored since we do not want to capture image?
                // For future impl. we can take advantage about how we can allieve existing job load. E.g. if I'm still rendering 50%, try to send this node the remaining parts?
                JobEvent::JobComplete => {} // Ignored, we're treated as a client node, waiting for new job request.
                _ => println!("Unhandle Job Event: {job_event:?}"),
            }
            // maybe move this inside Network code? Seems repeative in both cli and Tauri side of application here.
            NetEvent::InboundRequest { request, channel } => {
                if let Some(path) = self.providing_files.get(&request) {
                    println!("Sending file {path:?}");
                    controller.respond_file(std::fs::read(path).unwrap(), channel).await;
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

        loop {
            select! {
                Some(event) = event_receiver.recv() => self.handle_message(&mut client, event).await,
                // Some(msg) = from_cli.recv() => Self::handle_command(&mut controller, msg).await,
            }
        }
        // if somehow we were able to get out of the loop, we would best send a shutdown notice here.
        // client.shutdown().await;
    }
}
