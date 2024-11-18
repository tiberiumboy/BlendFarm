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
use crate::routes::remote_render::{create_node, delete_node, list_node, list_versions, ping_node};
use crate::routes::settings::{
    add_blender_installation, fetch_blender_installation, get_server_settings,
    list_blender_installation, remove_blender_installation, set_server_settings,
};
use models::app_state::AppState;
use services::network_service::NetworkService;
// use services::message::NetResponse;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
// use std::thread;
use tauri::AppHandle;

//TODO: Create a miro diagram structure of how this application suppose to work
// Need a mapping to explain how network should perform over intranet
// Need a mapping to explain how blender manager is used and invoked for the job

pub mod models;
pub mod routes;
pub mod services;

// I'm using this to make app handler accessible within this app. I will eventually find a better way to handle this.
// TODO: impl dependency injection?
// static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

// when the app starts up, I would need to have access to configs. Config is loaded from json file - which can be access by user or program - it must be validate first before anything,
fn client() {
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

    // I'm having problem trying to separate this call from client.
    // I want to be able to run either server _or_ client via a cli switch.
    // Would like to know how I can get around this?
    let server = NetworkService::new(true, 15000);

    let app_state = AppState {
        network: Arc::new(RwLock::new(server)),
    };

    let app = builder
        .manage(Mutex::new(app_state))
        .invoke_handler(tauri::generate_handler![
            create_node,
            create_job,
            delete_node,
            delete_job,
            list_node,
            list_jobs,
            list_versions,
            get_server_settings,
            set_server_settings,
            ping_node,
            add_blender_installation,
            list_blender_installation,
            remove_blender_installation,
            fetch_blender_installation,
        ])
        // it would be nice to figure out why this is causing so much problem?
        .build(tauri::generate_context!())
        .expect("Unable to build tauri app!");
    // APP_HANDLE.set(app.handle().clone()).unwrap();

    // match app.cli().matches() {
    //     // `matches` here is a Struct with { args, subcommand }.
    //     // `args` is `HashMap<String, ArgData>` where `ArgData` is a struct with { value, occurrences }.
    //     // `subcommand` is `Option<Box<SubcommandMatches>>` where `SubcommandMatches` is a struct with { name, matches }.
    //     // cargo tauri dev -- -- -c
    //     Ok(matches) => {
    //         dbg!(&matches);
    //         if matches.args.get("client").unwrap().occurrences >= 1 {
    //             // run client mode instead.
    //             spawn(run_client());
    //         }
    //     }
    //     Err(e) => {
    //         dbg!(e);
    //     }
    // };

    // As soon as the function goes out of scope, thread will be drop.
    // TODO: Find a better place to move this background process
    // Consider making a closure for this?
    /*
    let _thread = thread::spawn(move || {
        while let Ok(event) = listen.recv() {
            match event {
                NetResponse::Joined { socket } => {
                    println!("Net Response: [{}] joined!", socket);
                    let handle = APP_HANDLE.get().unwrap();
                    handle
                        .emit("node_joined", socket)
                        .expect("failed to emit node!");
                }
                NetResponse::Disconnected { socket } => {
                    println!("Net Response: [{}] disconnected!", socket);
                    let handle = APP_HANDLE.get().unwrap();
                    handle
                        .emit("node_left", socket)
                        .expect("failed to emit node!");
                }
                NetResponse::Info { socket, name } => {
                    println!("Net Response: [{}] - {}", socket, name);
                }
                NetResponse::Status { socket, status } => {
                    println!("Net Response: [{}] - {}", socket, status);
                }
                NetResponse::PeerList { addrs } => {
                    // TODO: Send a notification to front end containing peer data information
                    println!("Received peer list! {:?}", addrs);
                }
                // NetResponse::JobSent(job) => {
                //     let handle = APP_HANDLE.get().unwrap();
                //     handle
                //         .emit_to("job", "job_sent", job)
                //         .expect("failed to emit job!");
                // }
                NetResponse::ImageComplete(path) => {
                    let handle = APP_HANDLE.get().unwrap();
                    handle
                        .emit("image_update", path)
                        .expect("Fail to send completed image!");
                }
            }
        }
    }); */

    app.run(|_, _| {});
}

// not sure why I'm getting a lint warning about the mobile macro? Need to bug the dev and see if this macro has changed.
// #[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // TODO: Find a way to make use of Tauri cli commands to run as client.
    // TODO: It would be nice to include command line utility to let the user add blender installation from remotely.
    // The command line would take an argument of --add or -a to append local blender installation from the local machine to the configurations.
    client();
}