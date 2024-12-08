/*
Have a look into TUI for CLI status display window to show user entertainment on screen
https://docs.rs/tui/latest/tui/
*/
use super::blend_farm::BlendFarm;
use crate::models::{
    job::Job, message::{NetEvent, NetworkError}, network::NetworkController
};
use async_trait::async_trait;
use blender::{blender::Args, manager::Manager as BlenderManager};
use machine_info::Machine;
use semver::Version;
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
            active_job: Default::default() 
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
            NetEvent::NodeDiscovered(..) => println!("Cli is subscribe to SPEC topic, which should never happen!"), // should not happen? We're not subscribe to this topic.
            NetEvent::NodeDisconnected(_) => {} // don't care about this
            NetEvent::Render(peer_id, job) => {
                // first check and see if we have blender installation installed for this job.
                let blend_version: &Version = &job.project_file.as_ref();
                let status = format!("Checking for blender version {}", blend_version);
                controller.send_status(status).await;

                let mut manager = BlenderManager::load();
                let blender = manager
                    .fetch_blender(blend_version)
                    .expect("Fail to download blender!");

                let tmp_path = dirs::cache_dir().unwrap().join("Blender");
                let file_name = job.project_file.deref().file_name().unwrap().to_str().unwrap().to_string();
                let project_file = tmp_path.join(&job.project_file.deref());
                if !project_file.exists() {
                    // go fetch the project file from the network.
                    let _ = controller.request_file(peer_id.clone(), file_name).await;
                }
                let args = Args::new(project_file, tmp_path, job.mode);
                // TODO: Finish the rest of this implementation once we can transfer blend file from different machine.
                let _rx = blender.render(args);
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
        let system_info = format!("blendfarm/{}{}", consts::OS, &system.processor.brand );  
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
