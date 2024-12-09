/*
Have a look into TUI for CLI status display window to show user entertainment on screen
https://docs.rs/tui/latest/tui/
*/
use futures::{prelude::*, StreamExt};
use super::blend_farm::BlendFarm;
use crate::models::{
    job::Job,
    message::{NetEvent, NetworkError},
    network::NetworkController,
};
use async_trait::async_trait;
use blender::{blender::Args, manager::Manager as BlenderManager};
use machine_info::Machine;
use semver::Version;
use std::path::PathBuf;
use std::{env::consts, ops::Deref};
use tokio::{select, sync::mpsc::Receiver};

pub struct CliApp {
    machine: Machine,
    // job that this machine is busy working on.
    #[allow(dead_code)]
    active_job: Option<Job>,
}

impl Default for CliApp {
    fn default() -> Self {
        Self {
            machine: Machine::new(),
            active_job: Default::default(),
        }
    }
}

impl CliApp {

     /*

        TODO: Figure out what I was suppose to do with this file?

    //         NetEvent::Render(job) => {
        //             // Here we'll check the job -
        //             // TODO: It would be nice to check and see if there's any jobs currently running, otherwise put it in a poll?
        //             let project_file = job.project_file;
        //             let version: &Version = project_file.as_ref();
        //             let blender = self
        //                 .manager
        //                 .fetch_blender(version)
        //                 .expect("Should have blender installed?");
        //             let file_path: &Path = project_file.as_ref();
        //             let args = Args::new(file_path, job.output, job.mode);
        //             let rx = blender.render(args);
        // for this particular loop, let's extract this out to simplier function.
        // loop {
        //         if let Ok(msg) = rx.recv() {
        //             let msg = match msg {
        //                 Status::Idle => "Idle".to_owned(),
        //                 Status::Running { status } => status,
        //                 Status::Log { status } => status,
        //                 Status::Warning { message } => message,
        //                 Status::Error(err) => format!("{err:?}").to_owned(),
        //                 Status::Completed { result } => {
        //                     // we'll send the message back?
        //                     // net_service
        //                     // here we will state that the render is complete, and send a message to network service
        //                     // TODO: Find a better way to not use the `.clone()` method.
        //                     let msg = Command::FrameCompleted(
        //                         result.clone(),
        //                         job.current_frame,
        //                     );
        //                     let _ = net_service.send(msg).await;
        //                     let path_str = &result.to_string_lossy();
        //                     format!(
        //                         "Finished job frame {} at {path_str}",
        //                         job.current_frame
        //                     )
        //                     .to_owned()
        //                     // here we'll send the job back to the peer who requested us the job initially.
        //                     // net_service.swarm.behaviour_mut().gossipsub.publish( peer_id, )
        //                 }
        //             };
        //             println!("[Status] {msg}");
        //         }
        //             // }
        //         }
        // }

    */


    async fn handle_message(&mut self, controller: &mut NetworkController, event: NetEvent) {
        match event {
            NetEvent::OnConnected => controller.share_computer_info().await,
            NetEvent::NodeDiscovered(..) => { } // Ignored
            NetEvent::NodeDisconnected(_) => {} // ignored
            NetEvent::Render(job) => {
                // first check and see if we have blender installation installed for this job.
                // let status = format!("Checking for blender version {}", blend_version);
                let output = &controller.settings.render_dir;

                // There's simply way too many unwrap here, is there a better way to handle this?
                let file_name = job.get_file_name().unwrap().to_string();

                // create a path link where we think the file should be?
                let project_file = controller.settings.blend_dir.join(&file_name); // append the file name here instead.
                if !project_file.exists() {
                    
                    let providers = controller.get_providers(&file_name).await;
                    if providers.is_empty() {
                        // at this point we'll report back there's an error.
                        controller.send_status("Could not get source blender file!".to_owned()).await;
                        return;
                    }

                    let requests = providers.into_iter().map(|p| {
                        let mut client = controller.clone();
                        let file_name = file_name.clone();
                        async move { client.request_file(p, file_name).await }.boxed()
                    });

                    
                    let content = match futures::future::select_ok(requests)
                    .await {
                        Ok(data) => data.0,
                        Err(e) => {
                            controller.send_status("No provider return the file.".to_owned()).await;
                            return;
                        }
                    };

                    if let Err(e) = std::fs::write(project_file, content) {
                        controller.send_status("Could not save blender file to blender directory!".to_owned()).await;
                        return;
                    }

                    // // go fetch the project file from the network.
                    // if let Err(e) = controller.request_file(file_name).await {
                    //     eprintln!("Fail to request file from controller? {e:?}");
                    // }
                }
            
                match job.run(output).await {
                    Ok(rx) => {
                        loop {
                            select!{
                                Some(status) = rx.recv() => match status {
                                    blender::models::status::Status::Idle => controller.send_status("[Idle]".to_owned()).await,
                                    blender::models::status::Status::Running { status } => controller.send_status(format!("[Running] {status}")).await,
                                    blender::models::status::Status::Log { status } => controller.send_status(format!("[Log] {status}")).await,
                                    blender::models::status::Status::Warning { message } => controller.send_status(format!("[Warning] {message}")).await,
                                    blender::models::status::Status::Error(blender_error) => controller.send_status(format!(" {}")).await,
                                    blender::models::status::Status::Completed { frame, result } => {
                                        let file_name = result.file_name().unwrap().to_str().unwrap().to_string();
                                        controller.start_providing(file_name, result).await;
                                        controller
                                    },
                                }
                            }
                        }
                    }
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
