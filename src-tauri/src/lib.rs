/*
Developer blog:
- Had a brain fart trying to figure out some ideas allowing me to run this application as either client or server
    Originally thought of using Clap library to parse in input, but when I run `cargo tauri dev -- test` the application fail to compile due to unknown arguments when running web framework?
    This issue has been solved by alllowing certain argument to run. By default it will try to launch the client user interface of the application.
    Additionally, I need to check into the argument and see if there's a way we could just allow user to run --server without ui interface?
    Interesting thoughts for sure
    9/2/24
- Decided to rely on using Tauri plugin for cli commands and subcommands. Use that instead of clap. Since Tauri already incorporates Clap anyway.
- Had an idea that allows user remotely to locally add blender installation without using GUI interface,
    This would serves two purposes - allow user to expressly select which blender version they can choose from the remote machine and
    prevent multiple download instances for the node, in case the target machine does not have it pre-installed.
- Eventually, I will need to find a way to spin up a virtual machine and run blender farm on that machine to see about getting networking protocol working in place.
    This will allow me to do two things - I can continue to develop without needing to fire up a remote machine to test this and
    verify all packet works as intended while I can run the code in parallel to see if there's any issue I need to work overhead.
    This might be another big project to work over the summer to understand how network works in Rust.

[F] - find a way to allow GUI interface to run as client mode for non cli users.
[F] - consider using channel to stream data https://v2.tauri.app/develop/calling-frontend/#channels
[F] - Before release - find a way to add updater  https://v2.tauri.app/plugin/updater/
*/
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::routes::job::{create_job, delete_job, list_jobs};
use crate::routes::remote_render::{delete_node, import_blend, list_versions};
use crate::routes::settings::{
    add_blender_installation, fetch_blender_installation, get_server_settings,
    list_blender_installation, remove_blender_installation, set_server_settings,
};
use blender::manager::Manager as BlenderManager;
use clap::Parser;
use models::app_state::AppState;
use models::server_setting::ServerSetting;
use services::network_service::{Command, NetEvent, NetworkService};
use std::sync::{Arc, RwLock};
use tauri::{App, Emitter, Manager};
use tokio::select;
use tokio::sync::{
    mpsc::{self, Sender},
    Mutex,
};
use tracing_subscriber::EnvFilter;

//TODO: Create a miro diagram structure of how this application suppose to work
// Need a mapping to explain how network should perform over intranet
// Need a mapping to explain how blender manager is used and invoked for the job

pub mod models;
pub mod routes;
pub mod services;

// Create a builder to make Tauri application
fn config_tauri_builder(to_network: Sender<Command>) -> App {
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
            delete_node,
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

#[derive(Parser)]
struct Cli {
    #[arg(short, long, default_value = "false")]
    client: Option<bool>,
}

// not sure why I'm getting a lint warning about the mobile macro? Need to bug the dev and see if this macro has changed.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    let cli = Cli::parse();

    let mut net_service = NetworkService::new(60)
        .await
        .expect("Unable to start network service!");

    match cli.client {
        // TODO: Verify this function works as soon as VM is finish installing linux.
        Some(true) => {
            println!("Client is running");
            loop {
                select! {
                    Some(msg) = net_service.rx_recv.recv() => {
                        println!("[Client] Received message: {msg:?}");
                    }
                }
            }
        }
        _ => {
            let (to_network, mut from_ui) = mpsc::channel::<Command>(32);
            let app = config_tauri_builder(to_network);

            let app_handle = Arc::new(RwLock::new(app.app_handle().clone()));

            let _thread = tokio::spawn(async move {
                loop {
                    select! {
                        Some(msg) = from_ui.recv() => {
                            if let Err(e) = net_service.send(msg).await {
                                println!("Fail to send net service message: {e:?}");
                            }
                        }
                        Some(info) = net_service.rx_recv.recv() => match info {
                            NetEvent::Render(job) => println!("Job: {job:?}"),
                            NetEvent::Status(msg) => println!("Status: {msg:?}"),
                            NetEvent::NodeDiscovered(peer_id) => {
                                let handle = app_handle.read().unwrap();
                                // TODO: test this once we're online.
                                // println!("Node discovered: {peer_id:?}");
                                handle.emit("node_discover", peer_id).unwrap();
                            },
                            NetEvent::NodeDisconnected(peer_id) => {
                                let handle = app_handle.read().unwrap();
                                handle.emit("node_disconnect", peer_id).unwrap();
                            },
                        }
                    }
                }
            });

            app.run(|_, event| match event {
                tauri::RunEvent::Exit => {
                    println!("Program exit!");
                }
                tauri::RunEvent::ExitRequested { .. } => {
                    println!("Exit requested");
                }
                _ => {}
            });
        }
    };
}
