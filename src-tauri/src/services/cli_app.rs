use super::blend_farm::BlendFarm;
use crate::models::{
    message::{NetEvent, NetworkError},
    network::NetworkController,
};
use async_trait::async_trait;
use blender::{blender::Args, manager::Manager as BlenderManager};
use semver::Version;
use tokio::{select, sync::mpsc::Receiver};

#[derive(Default)]
pub struct CliApp;

impl CliApp {
    async fn handle_message(controller: &mut NetworkController, event: NetEvent) {
        match event {
            NetEvent::NodeDiscovered(_) => {
                controller.share_computer_info().await;
            }
            // receive network job queue.
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
                let file_name = job.project_file.file_name;
                let project_file = tmp_path.join(&file_name);
                if !project_file.exists() {
                    // go fetch the project file from the network.
                    let _ = controller.request_file(peer_id.clone(), file_name).await;
                }
                let args = Args::new(project_file, tmp_path, job.mode);
                // TODO: Finish the rest of this implementation once we can transfer blend file from different machine.
                let _rx = blender.render(args);
            }

            _ => println!("Received event from network: {event:?}"),
        }
    }
}

#[async_trait]
impl BlendFarm for CliApp {
    async fn run(
        &self,
        mut client: NetworkController,
        mut event_receiver: Receiver<NetEvent>,
    ) -> Result<(), NetworkError> {
        // may need to add one more for response back to network controller for job completion/status/updates
        // should be handle on it's own job thread? We'll see.
        loop {
            select! {
                Some(event) = event_receiver.recv() => Self::handle_message(&mut client, event).await,
                // Some(msg) = from_cli.recv() => Self::handle_command(&mut controller, msg).await,
            }
        }
    }
}
