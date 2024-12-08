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
