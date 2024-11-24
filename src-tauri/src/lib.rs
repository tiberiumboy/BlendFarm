/*
Developer blog:
- Had a brain fart trying to figure out some ideas allowing me to run this application as either client or server
Originally thought of using Clap library to parse in input, but when I run `cargo tauri dev -- test` the application fail to compile due to unknown arguments when running web framework?
This issue has been solved by alllowing certain argument to run. By default it will try to launch the client user interface of the application.
Additionally, I need to check into the argument and see if there's a way we could just allow user to run --server without ui interface?
Interesting thoughts for sure
9/2/24 - Decided to rely on using Tauri plugin for cli commands and subcommands. Use that instead of clap. Since Tauri already incorporates Clap anyway.

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
use crate::routes::remote_render::{delete_node, import_blend, list_node, list_versions};
use crate::routes::settings::{
    add_blender_installation, fetch_blender_installation, get_server_settings,
    list_blender_installation, remove_blender_installation, set_server_settings,
};
use blender::manager::Manager as BlenderManager;
use blender::models::home::BlenderHome;
use clap::Parser;
use models::app_state::AppState;
use models::server_setting::ServerSetting;
use services::network_service::{NetMessage, NetworkService};
use std::sync::{Arc, RwLock};
use tokio::select;
use tokio::sync::{mpsc, Mutex};
use tracing_subscriber::EnvFilter;

//TODO: Create a miro diagram structure of how this application suppose to work
// Need a mapping to explain how network should perform over intranet
// Need a mapping to explain how blender manager is used and invoked for the job

pub mod models;
pub mod routes;
pub mod services;

// when the app starts up, I would need to have access to configs. Config is loaded from json file - which can be access by user or program - it must be validate first before anything,
fn client(server_settings: ServerSetting, net_service: Mutex<NetworkService>) {
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

    // TODO: Might combine manager and source together?
    let manager = Arc::new(RwLock::new(BlenderManager::load()));
    let blender_source = Arc::new(RwLock::new(
        BlenderHome::new()
            .expect("Unable to connect to blender.org, are you connect to the internet?"),
    ));
    let setting = Arc::new(RwLock::new(server_settings));
    let (to_network, mut from_ui) = mpsc::channel::<NetMessage>(32);

    // Do consider adding blender manager and blender home in app state instead.
    let app_state = AppState {
        manager,
        to_network,
        blender_source,
        setting,
        jobs: Vec::new(),
    };

    let mut_app_state = Mutex::new(app_state);

    let app = builder
        .manage(mut_app_state)
        .invoke_handler(tauri::generate_handler![
            create_job,
            delete_node,
            delete_job,
            list_node,
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
        .expect("Unable to build tauri app!");

    // spin up a new thread to handle message queue from App UI to Network services
    let _thread = tokio::spawn(async move {
        loop {
            select! {
            // let's make sure this works!
            Some(msg) = from_ui.recv() => {
                println!("{msg:?}");
                // let mut service = net_service.lock().await;
                // let _ = service. (msg).await;
            }

            // Ok(info) = net_service.from_network.recv() {
            //     //     // process event from network, e.g. if new peer joins, we should send a notification to app.
            //     // }
            //     println!("{:?}", info);
            // }
            }
        }
    });

    app.run(|_, _| {});
    // TODO: This could be a problem. I want to make sure I could exit the application successfully.
    // this never gets called?
    println!("After run");
}

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    client: Option<bool>, // TOOD: Find a way to provide default value?
}

// not sure why I'm getting a lint warning about the mobile macro? Need to bug the dev and see if this macro has changed.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    let cli = Cli::parse();

    // Spin up network service.
    let net_service = NetworkService::new(60)
        .await
        .expect("Unable to start network service!");
    let service = Mutex::new(net_service);

    // read CLI commands here.
    match cli.client {
        Some(true) => {
            let thread = service.lock().await;
            thread.as_ref().is_finished();
            return;
        }
        _ => {
            let config = ServerSetting::load();
            client(config, service);
        }
    }
}
