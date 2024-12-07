use crate::{
    models::{
        app_state::AppState,
        job::Job,
        message::{NetEvent, NetworkError},
        network::NetworkController,
        server_setting::ServerSetting,
    },
    routes::{job::*, remote_render::*, settings::*},
};
use blender::manager::Manager as BlenderManager;
use std::{path::PathBuf, sync::Arc};
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
pub struct DisplayApp {}

impl DisplayApp {
    // Create a builder to make Tauri application
    fn config_tauri_builder(to_network: Sender<UiCommand>) -> App {
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
            .expect("Unable to build tauri app!")
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
                let project_file = &job.project_file;
                let file_name = project_file.get_file_name().to_string();
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
        client: &mut NetworkController,
        event: NetEvent,
        app_handle: Arc<RwLock<AppHandle>>,
    ) {
        match event {
            NetEvent::Render(peer_id, job) => {
                println!("Receive Job from peerid {peer_id:?}: {job:?}")
            }
            NetEvent::Status(peer_id, msg) => println!("Status from {peer_id} : {msg:?}"),
            NetEvent::NodeDiscovered(peer_id) => {
                println!("Node Discovered {peer_id}");
                let handle = app_handle.read().await;
                handle.emit("node_discover", peer_id.to_base58()).unwrap();
                client.share_computer_info().await;
            }
            NetEvent::NodeDisconnected(peer_id) => {
                println!("Node disconnected {peer_id}");
                let handle = app_handle.read().await;
                handle.emit("node_disconnect", peer_id.to_base58()).unwrap();
            }
            NetEvent::Identity(peer_id, comp_spec) => {
                println!("Received node identity for id {peer_id} : {comp_spec:?}");
                let handle = app_handle.read().await;
                // would this be ideal to store a bytes instead?
                // TODO: Change this to target Node by peer_id - See
                handle
                    .emit("node_identity", (peer_id.to_base58(), comp_spec))
                    .unwrap();
            }
            _ => println!("{:?}", event),
        }
    }
}

#[async_trait::async_trait]
impl BlendFarm for DisplayApp {
    async fn run(
        &self,
        mut client: NetworkController,
        mut event_receiver: Receiver<NetEvent>,
    ) -> Result<(), NetworkError> {
        // this channel is used to send command to the network, and receive network notification back.
        let (to_network, mut from_ui) = mpsc::channel(32);

        // we send the sender to the tauri builder - which will send commands to "from_ui".
        let app = Self::config_tauri_builder(to_network);

        // create a safe and mutable way to pass application handler to send notification from network event.
        let app_handle = Arc::new(RwLock::new(app.app_handle().clone()));

        // create a background loop to send and process network event
        spawn(async move {
            loop {
                select! {
                    Some(msg) = from_ui.recv() => Self::handle_ui_command(&mut client, msg, app_handle.clone()).await,
                    Some(event) = event_receiver.recv() => Self::handle_net_event(&mut client, event, app_handle.clone()).await,
                }
            }
        });

        // Run the app.
        app.run(|_, event| {
            match event {
                // TODO: find a way to spawn the network listener thread inside here?
                tauri::RunEvent::Ready => println!("Application is ready!"),
                _ => {}
            }
        });

        Ok(())
    }
}
