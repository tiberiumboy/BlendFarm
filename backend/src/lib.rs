/*
    Developer blog:
    - Had a brain fart trying to figure out some ideas allowing me to run this application as either client or server
        Originally thought of using Clap library to parse in input, but when I run `cargo tauri dev -- test` the application fail to compile due to unknown arguments when running web framework?
        This issue has been solved by alllowing certain argument to run. By default it will try to launch the client user interface of the application.
        Additionally, I need to check into the argument and see if there's a way we could just allow user to run --server without ui interface?
        Interesting thoughts for sure
        9/2/24 - Decided to rely on using Tauri plugin for cli commands and subcommands. Use that instead of clap.
    - Had an idea that allows user remotely to locally add blender installation without using GUI interface,
        This would serves two purposes - allow user to expressly select which blender version they can choose from the remote machine and
        prevent multiple download instances for the node, in case the target machine does not have it pre-installed.
    - Eventually, I will need to find a way to spin up a virtual machine and run blender farm on that machine to see about getting networking protocol working in place.
        This will allow me to do two things - I can continue to develop without needing to fire up a remote machine to test this and
        verify all packet works as intended while I can run the code in parallel to see if there's any issue I need to work overhead.
        This might be another big project to work over the summer to understand how network works in Rust.

[F] - Take a look into multiple producer single consumer (std::sync::mpsc):
        See how we can handle newly connected node or other node property into subscribable
        state to send notification to tauri front end.


[F] - find a way to allow GUI interface to run as client mode for non cli users.

*/

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::controllers::remote_render::{
    create_job, create_node, delete_job, delete_node, delete_project, edit_node, import_project,
    list_job, list_node, list_projects, list_versions, ping_node,
};
use crate::controllers::settings::{
    add_blender_installation, get_server_settings, list_blender_installation,
    remove_blender_installation, set_server_settings,
};
use crate::models::{client::Client, data::Data, server::Server};
use models::message::NetResponse;
use std::{sync::Mutex, thread};
use tauri_plugin_cli::CliExt;

pub mod controllers;
pub mod models;
pub mod services;

// when the app starts up, I would need to have access to configs. Config is loaded from json file - which can be access by user or program - it must be validate first before anything,
fn client() {
    let data = Data::default();
    // I would like to find a better way to update or append data to render_nodes,
    // but I need to review more context about handling context like this in rust.
    // I understand Mutex, but I do not know if it's any safe to create pointers inside data struct from mutex memory.
    // "Do not communicate with shared memory"\
    let ctx = Mutex::new(data);

    let mut server = Server::new(1500);
    let listen = server.rx_recv.take().unwrap();

    // As soon as the function goes out of scope, thread will be drop.
    let _thread = thread::spawn(move || {
        while let Ok(event) = listen.recv() {
            match event {
                NetResponse::Joined { socket } => {
                    println!("Net Response: [{}] joined!", socket);
                }
                NetResponse::Disconnected { socket } => {
                    println!("Net Response: [{}] disconnected!", socket);
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
            }
        }
    });

    // how do I receive the events?
    let m_server = Mutex::new(server);

    // let client = CustomMenuItem::new("client_mode".to_string(), "Run as Client");
    // let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    // // let submenu = Submenu::new("File", Menu::new().add_item(quit).add_item(close));
    // let menu = Menu::new().add_item(quit).add_item(client);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_cli::init())
        .setup(|app| {
            //     // now that we know what the app version is - we can use it to set our global version variable, as our main node reference.
            //     // it would be nice to include version number in title bar of the app.
            //     println!("{}", app.package_info().version);
            match app.cli().matches() {
                // `matches` here is a Struct with { args, subcommand }.
                // `args` is `HashMap<String, ArgData>` where `ArgData` is a struct with { value, occurrences }.
                // `subcommand` is `Option<Box<SubcommandMatches>>` where `SubcommandMatches` is a struct with { name, matches }.
                Ok(matches) => {
                    dbg!(&matches);
                    if matches.args.get("client").unwrap().occurrences > 1 {
                        // run client mode instead.
                        println!("Running client!");
                        let _ = Client::new();
                    }
                }
                Err(e) => {
                    dbg!(e);
                }
            };
            Ok(())
        })
        // https://docs.rs/tauri/1.6.8/tauri/struct.Builder.html#method.manage
        // It is indeed possible to have more than one manage - which I will be taking advantage over how I can share and mutate configuration data across this platform.
        .manage(ctx)
        .manage(m_server)
        // .menu(menu)
        // .on_menu_event(|event| match event.menu_item_id() {
        //     "quit" => {
        //         std::process::exit(0);
        //     }
        //     "client_mode" => {
        //         println!("Run this program as client mode - How should the GUI change for this?");
        //         // Hide the application to traybar - until the user decided to restart as a server.
        //         let _client = Client::new();
        //     }
        //     _ => {}
        // })
        .invoke_handler(tauri::generate_handler![
            import_project,
            create_node,
            create_job,
            delete_node,
            delete_project,
            delete_job,
            edit_node,
            list_node,
            list_projects,
            list_job,
            list_versions,
            get_server_settings,
            set_server_settings,
            ping_node,
            add_blender_installation,
            list_blender_installation,
            remove_blender_installation,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    // the idea here is that once the app goes out of scope, it is no longer up and running. I should then terminate the job.
    // TODO - how do I keep the service alive? Is it possible to run the app as service mode? cli mode? Would be interesting.
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // TODO: Find a way to make use of Tauri cli commands to run as client.
    // TODO: It would be nice to include command line utility to let the user add blender installation from remotely.
    // The command line would take an argument of --add or -a to append local blender installation from the local machine to the configurations.
    client();
}
