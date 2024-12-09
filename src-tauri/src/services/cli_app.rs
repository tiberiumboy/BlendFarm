/*
Have a look into TUI for CLI status display window to show user entertainment on screen
https://docs.rs/tui/latest/tui/
*/
use futures::prelude::*;
use super::blend_farm::BlendFarm;
use crate::models::{
    job::Job,
    message::{NetEvent, NetworkError},
    network::{NetworkController, JOB},
};
use async_trait::async_trait;
use blender::models::status::Status;
use machine_info::Machine;
use std::{collections::HashMap, env::consts, path::PathBuf};
use tokio::{select, sync::mpsc::Receiver};

pub struct CliApp {
    machine: Machine,
    // job that this machine is busy working on.
    #[allow(dead_code)]
    active_job: Option<Job>,
    provider_files : HashMap<String, PathBuf>
}

impl Default for CliApp {
    fn default() -> Self {
        Self {
            machine: Machine::new(),
            active_job: Default::default(),
            provider_files: Default::default(),
        }
    }
}

impl CliApp {
    async fn handle_message(&mut self, controller: &mut NetworkController, event: NetEvent) {
        match event {
            NetEvent::OnConnected => controller.share_computer_info().await,
            NetEvent::NodeDiscovered(..) => { } // Ignored
            NetEvent::NodeDisconnected(_) => {} // ignored
            NetEvent::Render(mut job) => {
                let status = format!("Receive render job [{}]", job.as_ref());
                controller.send_status(status).await; 
                
                let output = controller.settings.render_dir.clone();
                let file_name = job.get_file_name().unwrap().to_string();
                
                // create a path link where we think the file should be?
                let project_file = controller.settings.blend_dir.join(&file_name); // append the file name here instead.
                controller.send_status(format!("Checking for project file {:?}", &project_file)).await;

                if !project_file.exists() {
                    println!("Project file do not exist, asking to download from host: {:?}", &project_file);    
                    let providers = controller.get_providers(&file_name).await;
                    if providers.is_empty() {
                        // at this point we'll report back there's an error.
                        println!("Unable to find provider that have this file! Aborting...");
                        controller.send_status("Could not get source blender file!".to_owned()).await;
                        return;
                    }
                    
                    println!("Found providers!");
                    let requests = providers.into_iter().map(|p| {
                        let mut client = controller.clone();
                        let file_name = file_name.clone();
                        async move { client.request_file(p, file_name).await }.boxed()
                    });

                    println!("Awaiting future result!");
                    let content = match futures::future::select_ok(requests)
                    .await {
                        Ok(data) => data.0,
                        Err(e) => {
                            controller.send_status(format!("No provider return the file. {e:?}")).await;
                            return;
                        }
                    };
                    println!("Downloading project file...");
                    if let Err(e) = std::fs::write(project_file, content) {
                        controller.send_status(format!("Could not save blender file to blender directory! {e:?}")).await;
                        return;
                    }

                    // // go fetch the project file from the network.
                    // if let Err(e) = controller.request_file(file_name).await {
                    //     eprintln!("Fail to request file from controller? {e:?}");
                    // }
                }
                
                // run the job!
                match job.run(output).await {
                    Ok(rx) => {
                        loop {
                            if let Ok(status) = rx.recv() {
                                match status {
                                    Status::Idle => controller.send_status("[Idle]".to_owned()).await,
                                    Status::Running { status } => controller.send_status(format!("[Running] {status}")).await,
                                    Status::Log { status } => controller.send_status(format!("[Log] {status}")).await,
                                    Status::Warning { message } => controller.send_status(format!("[Warning] {message}")).await,
                                    Status::Error(blender_error) => controller.send_status(format!("[ERR] {blender_error:?}")).await,
                                    Status::Completed { result, .. } => {
                                        let file_name = result.file_name().unwrap().to_str().unwrap().to_string();
                                        self.provider_files.insert(file_name.clone(), result);
                                        controller.start_providing(file_name).await;
                                        
                                        // I think I need to add one more implementation to notify complete image result with frame count
                                        controller.request_job(None).await;
                                        break;
                                    },
                                };
                            }
                        }
                    },
                    Err(e) => {
                        controller.request_job(Some(e)).await
                    }
                };
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
        let system = self.machine.system_info();
        let system_info = format!("blendfarm/{}{}", consts::OS, &system.processor.brand);
        client.subscribe_to_topic(system_info).await;
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
