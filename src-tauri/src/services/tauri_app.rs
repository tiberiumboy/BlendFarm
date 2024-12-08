use crate::{
    models::{
        app_state::AppState, computer_spec::ComputerSpec, job::Job, message::{NetEvent, NetworkError}, network::{NetworkController, HEARTBEAT, SPEC, STATUS}, server_setting::ServerSetting
    },
    routes::{job::*, remote_render::*, settings::*},
};
use blender::manager::Manager as BlenderManager;
use libp2p::PeerId;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tauri::{self, App, AppHandle, Emitter, Manager};
use tokio::{
    select, spawn,
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex, RwLock,
    },
};
use uuid::Uuid;

// This UI Command represent the top level UI that user clicks and interface with.
#[derive(Debug)]
pub enum UiCommand {
    StartJob(Job),
    StopJob(Uuid),
    UploadFile(PathBuf),
}

use super::blend_farm::BlendFarm;

#[derive(Default)]
pub struct TauriApp {
    peers: HashMap<PeerId, ComputerSpec>,
    // jobs: Vec<Job>,
}

impl TauriApp {
    // Create a builder to make Tauri application
    fn config_tauri_builder(to_network: Sender<UiCommand>) -> Result<App, tauri::Error> {
        let server_settings = ServerSetting::load();

        // I would like to find a better way to update or append data to render_nodes,
        // "Do not communicate with shared memory"
        let builder = tauri::Builder::default()
            .plugin(tauri_plugin_cli::init())
            .plugin(tauri_plugin_os::init())
            .plugin(tauri_plugin_fs::init())
            .plugin(tauri_plugin_persisted_scope::init())
            .plugin(tauri_plugin_shell::init())
            .plugin(tauri_plugin_dialog::init())
            .setup(|_| Ok(()));

        let manager = Arc::new(RwLock::new(BlenderManager::load()));
        let setting = Arc::new(RwLock::new(server_settings));

        // here we're setting the sender command to app state before the builder.
        let app_state = AppState {
            manager,
            to_network,
            setting,
            jobs: Vec::new(),
        };

        let mut_app_state = Mutex::new(app_state);

        builder
            .manage(mut_app_state)
            .invoke_handler(tauri::generate_handler![
                create_job,
                delete_job,
                list_jobs,
                list_versions,
                import_blend,
                get_server_settings,
                set_server_settings,
                add_blender_installation,
                list_blender_installation,
                remove_blender_installation,
                fetch_blender_installation,
            ])
            .build(tauri::generate_context!())
    }

    // command received from UI
    async fn handle_ui_command(
        client: &mut NetworkController,
        cmd: UiCommand,
        _app_handle: Arc<RwLock<AppHandle>>,
    ) {
        match cmd {
            UiCommand::StartJob(job) => {
                // first make the file available on the network
                let file_name = job.project_file.file_name().unwrap().to_str().unwrap().to_string();
                client.start_providing(file_name).await;
                client.send_network_job(job).await;
            }
            UiCommand::UploadFile(path) => {
                if let Some(file_name) = path.file_name() {
                    client
                        .start_providing(file_name.to_str().unwrap().to_string())
                        .await;
                }
            }
            UiCommand::StopJob(id) => {
                todo!(
                    "Impl how to send a stop signal to stop the job and remove the job from queue {id:?}"
                );
            }
        }
    }

    // commands received from network
    async fn handle_net_event(
        &mut self,
        _client: &mut NetworkController,
        event: NetEvent,
        app_handle: Arc<RwLock<AppHandle>>,
    ) {
        match event {
            NetEvent::Status(peer_id, msg) => {
                let handle = app_handle.read().await;
                handle.emit("node_status", (peer_id.to_base58(), msg)).unwrap();
            }
            NetEvent::NodeDiscovered(peer_id, comp_spec) => {
                let handle = app_handle.read().await;
                // println!("Received Node identity from computers! {:?}", &comp_spec);
                handle
                    .emit("node_discover", (peer_id.to_base58(), comp_spec.clone()))
                    .unwrap();
                self.peers.insert(peer_id, comp_spec);
            }
            // don't think there's a way for me to get this working?
            NetEvent::NodeDisconnected(peer_id) => {
                // println!("Received node disconnection: {peer_id}");
                let handle = app_handle.read().await;
                handle.emit("node_disconnect", peer_id.to_base58()).unwrap();
            }
            NetEvent::RequestJob => {
                // a peer on the network is asking for a job to work on.
                // TODO: implment a way to notify job manager to request a new rendering job...?
            }
            _ => println!("{:?}", event),
        }
    }
}

#[async_trait::async_trait]
impl BlendFarm for TauriApp {
    async fn run(
        mut self,
        mut client: NetworkController,
        mut event_receiver: Receiver<NetEvent>,
    ) -> Result<(), NetworkError> {

        // for application side, we will subscribe to message event that's important to us to intercept.
        client.subscribe_to_topic(SPEC.to_owned()).await;
        client.subscribe_to_topic(HEARTBEAT.to_owned()).await;
        client.subscribe_to_topic(STATUS.to_owned()).await;
        
        // this channel is used to send command to the network, and receive network notification back.
        let (to_network, mut from_ui) = mpsc::channel(32);

        // we send the sender to the tauri builder - which will send commands to "from_ui".
        let app = Self::config_tauri_builder(to_network)
            .expect("Fail to build tauri app - Is there an active display session running?");

        // create a safe and mutable way to pass application handler to send notification from network event.
        let app_handle = Arc::new(RwLock::new(app.app_handle().clone()));

        // create a background loop to send and process network event
        spawn(async move {
            loop {
                select! {
                    Some(msg) = from_ui.recv() => Self::handle_ui_command(&mut client, msg, app_handle.clone()).await,
                    Some(event) = event_receiver.recv() => self.handle_net_event(&mut client, event, app_handle.clone()).await,
                }
            }
        });

        // Run the app.
        app.run(|_app_handle, event| {
            match event {
                // TODO: find a way to spawn the network listener thread inside here?
                tauri::RunEvent::Ready => {
                    println!("Application is ready!");
                    // can't do this either :(
                    // for peer in &self.peers {
                    //     app_handle.emit("node_discover", (peer.0.to_base58(), peer.1));
                    // }
                }
                _ => {}
            }
        });

        Ok(())
    }
}
