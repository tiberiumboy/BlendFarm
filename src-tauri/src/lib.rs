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

- I noticed that some of the function are getting called twice. Check and see what's going on with React UI side of things
    Research into profiling front end ui to ensure the app is not invoking the same command twice.

[F] - find a way to allow GUI interface to run as client mode for non cli users.
[F] - consider using channel to stream data https://v2.tauri.app/develop/calling-frontend/#channels
[F] - Before release - find a way to add updater  https://v2.tauri.app/plugin/updater/
*/
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::routes::job::{create_job, delete_job, list_jobs};
use crate::routes::remote_render::{import_blend, list_versions};
use crate::routes::settings::{
    add_blender_installation, fetch_blender_installation, get_server_settings,
    list_blender_installation, remove_blender_installation, set_server_settings,
};
use blender::models::status::Status;
use blender::{manager::Manager as BlenderManager, models::args::Args};
use clap::Parser;
use models::app_state::AppState;
use models::message::{Command, NetEvent};
use models::network::{Host, NetworkService};
use models::server_setting::ServerSetting;
use semver::Version;
// use services::network_service::{Command, NetEvent, NetworkService};
use std::path::Path;
use std::sync::{Arc, RwLock};
use tauri::{App, Manager};
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

    // Just realize that libp2p mdns won't run offline mode?
    // TODO: Find a way to connect to local host itself regardless offline/online mode.
    let mut net_service = NetworkService::new(60)
        .await
        .expect("Unable to start network service!");

    match cli.client {
        Some(true) => {
            // TODO: Extract this into separate model
            let mut manager = BlenderManager::load();
            println!("Client is running");
            loop {
                select! {
                    Some(msg) = net_service.rx_recv.recv() => match msg {
                        NetEvent::Render(job) => {
                            // Here we'll check the job -
                            // TODO: It would be nice to check and see if there's any jobs currently running, otherwise put it in a poll?
                            let project_file = job.project_file;
                            let version: &Version = project_file.as_ref();
                            let blender = manager.fetch_blender(version).expect("Should have blender installed?");
                            let file_path: &Path = project_file.as_ref();
                            let args = Args::new(file_path, job.output, job.mode);
                            let rx = blender.render(args);

                            loop {
                                if let Ok(msg) = rx.recv() {
                                        let msg = match msg {
                                            Status::Idle => "Idle".to_owned(),
                                            Status::Running { status } => status,
                                            Status::Log { status } => status,
                                            Status::Warning { message } => message,
                                            Status::Error(err) => format!("{err:?}").to_owned(),
                                            Status::Completed { result } => {
                                                // we'll send the message back?
                                                // net_service
                                                // here we will state that the render is complete, and send a message to network service
                                                // TODO: Find a better way to not use the `.clone()` method.
                                                let msg = Command::FrameCompleted(result.clone(), job.current_frame);
                                                let _ = net_service.send(msg).await;
                                                let path_str = &result.to_string_lossy();
                                                format!("Finished job frame {} at {path_str}", job.current_frame).to_owned()
                                            },
                                        };
                                        println!("[Status] {msg}");
                                    }

                            }

                        },
                        NetEvent::Status(s) => println!("[Client] Status: {s}"),
                        NetEvent::NodeDiscovered(peer_id) => println!("Node discovered!: {peer_id}"),
                        // For some reason when we exit the application, this doesn't get called?
                        NetEvent::NodeDisconnected(peer_id) => println!("Node disconnected!: {peer_id}"),
                        NetEvent::Identity(comp_spec) => {
                            println!("Node Identity received: {comp_spec:?}");
                        }
                    }
                }
            }
        }
        _ => {
            // channel is created to send the receiver to the builder
            // the sender is then pass on to the network service and host loop event
            let (to_network, from_ui) = mpsc::channel::<Command>(32);
            let app = config_tauri_builder(to_network);

            // So I think this is where I last left off from? Trying to simplify this loop process by pushing into a separate struct container
            // I ran into a problem where I cannot move Receiver<Command> into the handler down below.
            // To resolve this I was going to create a new struct object that holds the Receiver<Command> and then actively read it inside the loop while the app is running.

            let mut host = Host::new(net_service, from_ui);
            let app_handle = Arc::new(RwLock::new(app.app_handle().clone()));
            // Problem here - I need to start capturing the data as soon as the app starts -
            // but I cannot move host object because receiver does not implement clone copy(), cannot move the struct inside this closure?
            let _thread = tokio::spawn(async move {
                host.run(app_handle).await;
            });

            app.run(|_, event| match event {
                tauri::RunEvent::Ready => {
                    // TODO: find a way to start receiving the client handle here instead?
                }
                tauri::RunEvent::Exit => {
                    // There should be a call to notify all other peers the GUI is shutting down.
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
