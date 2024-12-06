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
use blender::manager::Manager as BlenderManager;
use clap::Parser;
use models::app_state::AppState;
use models::message::{NetCommand, NetEvent};
use models::network::{Client, NetworkService};
use models::server_setting::ServerSetting;
use std::sync::{Arc, RwLock};
use tauri::{App, RunEvent};
use tokio::select;
use tokio::sync::{
    mpsc::{self, Sender},
    Mutex,
};
use tracing_subscriber::EnvFilter;

// TODO: Create a miro diagram structure of how this application suppose to work
// Need a mapping to explain how network should perform over intranet
// Need a mapping to explain how blender manager is used and invoked for the job

pub mod models;
pub mod routes;

#[derive(Parser)]
struct Cli {
    #[arg(short, long, default_value = "false")]
    client: Option<bool>,
}

struct DisplayApp {}

impl DisplayApp {
    // Create a builder to make Tauri application
    fn config_tauri_builder(to_network: Sender<NetCommand>) -> App {
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
    async fn handle_ui_command(client: &mut Client, cmd: NetCommand) {
        match cmd {
            NetCommand::SendIdentity => client.share_computer_info().await,
            NetCommand::StartJob(job) => {
                // first make the file available on the network
                let project_file = job.project_file;
                let file_name = project_file.get_file_name().to_string();
                client.start_providing(file_name).await;
            }
            _ => println!("Unhandled cmd! {cmd:?}"),
        }
    }

    // commands received from network
    async fn handle_net_event(client: &mut Client, event: NetEvent) {
        match event {
            NetEvent::Render(job) => println!("Receive Job: {job:?}"),
            NetEvent::Status(peer_id, msg) => println!("Status from {peer_id} : {msg:?}"),
            NetEvent::NodeDiscovered(peer_id) => {
                println!("Node Discovered {peer_id}");
                client.share_computer_info().await;
            }
            NetEvent::NodeDisconnected(peer_id) => {
                println!("Node disconnected {peer_id}");
                // let handle = app_handle.read().unwrap();
                // handle.emit("node_disconnect", peer_id).unwrap();
            }
            NetEvent::Identity(peer_id, comp_spec) => {
                println!("Received node identity for id {peer_id} : {comp_spec:?}");
                // let handle = app_handle.read().unwrap();
                // handle.emit("node_identity", (peer_id, comp_spec)).unwrap();
            } // _ => println!("{:?}", event),
        }
    }

    async fn handle_net_command(cmd: NetCommand) {
        todo!("handle incoming network commands {cmd:?}");
    }

    pub async fn run() {
        // this channel is used to send command to the network, and receive network notification back.
        let (to_network, mut from_ui) = mpsc::channel(32);
        // TODO: figure out how I can use this cmd_sender?
        let (mut net_service, mut client, mut event_receiver) = NetworkService::new()
            .await
            .expect("Unable to create network service!");

        // we send the sender to the tauri builder - which will send commands to "from_ui".
        let app = Self::config_tauri_builder(to_network);
        // let client = Arc::new(RwLock::new(client));

        let _task = tokio::spawn(async move {
            let _ = net_service.run().await;
            loop {
                select! {
                    Some(msg) = from_ui.recv() => Self::handle_ui_command(&mut client, msg).await,
                    event = event_receiver.recv() => match event {
                        Some(event) => Self::handle_net_event(&mut client, event).await,
                        None => break,
                    }
                }
            }
        });

        app.run(|_, event| match event {
            RunEvent::Ready => {
                // TODO: find a way to start receiving the client handle here instead?
            }
            // tauri::RunEvent::Exit => {
            //     // There should be a call to notify all other peers the GUI is shutting down.
            //     println!("Program exit!");
            // }
            RunEvent::ExitRequested { .. } => {
                // sender.send(NetCommand::Shutdown); // instruct to shutdown loops
            }
            tauri::RunEvent::WindowEvent { .. } => {} // invokes when program moves, gain/lose focus
            tauri::RunEvent::MainEventsCleared => {}  // this spam the console log.
            _ => println!("Program event: {event:?}"),
        });
    }

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
}

struct HeadlessClient {}

impl HeadlessClient {
    pub async fn run() {
        // TODO: Handle this inside headlessclient implementation
        let (mut net_service, _sender, _receiver) = NetworkService::new()
            .await
            .expect("Fail to start network service!");

        net_service.run().await;
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    let cli = Cli::parse();

    match cli.client {
        // run as client mode.
        Some(true) => HeadlessClient::run().await,
        // run as GUI mode.
        _ => DisplayApp::run().await,
    };
}
