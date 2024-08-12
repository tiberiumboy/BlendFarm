/*
    Developer blog:
    - Had a brain fart trying to figure out some ideas allowing me to run this application as either client or server
        Originally thought of using Clap library to parse in input, but when I run `cargo tauri dev -- test` the application fail to compile due to unknown arguments when running web framework?
        This issue has been solved by alllowing certain argument to run. By default it will try to launch the client user interface of the application.
        Additionally, I need to check into the argument and see if there's a way we could just allow user to run --server without ui interface?
        Interesting thoughts for sure
    - Had an idea that allows user remotely to locally add blender installation without using GUI interface,
        This would serves two purposes - allow user to expressly select which blender version they can choose from the remote machine and
        prevent multiple download instances for the node, in case the target machine does not have it pre-installed.
    - Eventually, I will need to find a way to spin up a virtual machine and run blender farm on that machine to see about getting networking protocol working in place.
        This will allow me to do two things - I can continue to develop without needing to fire up a remote machine to test this and
        verify all packet works as intended while I can run the code in parallel to see if there's any issue I need to work overhead.
        This might be another big project to work over the summer to understand how network works in Rust.

[F] - Take a look into multiple producer single consumer - See how we can handle newly connected node or other node property into subscribable state to send to tauri front end.
        It would be nice to dynamically see new node appended to the list as they become active on the network. Otherwise, we would have the user to manually type the ip address and
        hopefully the node can connect to the host.
*/

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::controllers::remote_render::{
    create_job, create_node, delete_job, delete_node, delete_project, edit_node, import_project,
    list_job, list_node, list_projects, list_versions, ping_node,
};
use crate::controllers::settings::{
    add_blender_installation, get_server_settings, list_blender_installation,
    remove_blender_installation,
};
use crate::models::{data::Data, server::Server};
use clap::Parser;
use cli::Cli;
use models::client::Client;
use models::message::NetResponse;
use std::thread;
use std::{env, io::Result, sync::Mutex};
use tauri::{generate_handler, CustomMenuItem, Menu, MenuItem, Submenu};

pub mod cli;
pub mod controllers;
pub mod models;
pub mod services;

// when the app starts up, I would need to have access to onfigs. Config is loaded from json file - which can be access by user or program - it must be validate first before anything,
// I will have to create a manager struct -this is self managed by user action. e.g. new node, edit project files, delete jobs, etc.
fn client() {
    let data = Data::default();
    // I would like to find a better way to update or append data to render_nodes,
    // but I need to review more context about handling context like this in rust.
    // I understand Mutex, but I do not know if it's any safe to create pointers inside data struct from mutex memory.
    // "Do not communicate with shared memory"\
    let ctx = Mutex::new(data);

    let mut server = Server::new(1500);

    let listen = server.rx_recv.take().unwrap();

    // need to figure out how I can destroy this thread?
    // unless I could just return this object with the thread attached? Send it up to main for responsibility
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
            }
        }
    });

    // so does this just becomes a loop problem then?
    // how do I receive the events?
    let m_server = Mutex::new(server);

    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let close = CustomMenuItem::new("close".to_string(), "Close");
    let submenu = Submenu::new("File", Menu::new().add_item(quit).add_item(close));
    let menu = Menu::new()
        .add_native_item(MenuItem::Copy)
        .add_item(CustomMenuItem::new("hide", "Hide"))
        .add_submenu(submenu);

    // why I can't dive into implementation details here?
    tauri::Builder::default()
        // https://docs.rs/tauri/1.6.8/tauri/struct.Builder.html#method.manage
        // It is indeed possible to have more than one manage - which I will be taking advantage over how I can share and mutate configuration data across this platform.
        .manage(ctx)
        .manage(m_server)
        .menu(menu)
        // .setup(|app| {
        //     // now that we know what the app version is - we can use it to set our global version variable, as our main node reference.
        //     // it would be nice to include version number in title bar of the app.
        //     println!("{}", app.package_info().version);
        //     Ok(())
        // })
        .invoke_handler(generate_handler![
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

fn main() -> Result<()> {
    let args = Cli::parse();
    // TODO: It would be nice to include command line utility to let the user add blender installation from remotely.
    // The command line would take an argument of --add or -a to append local blender installation from the local machine to the configurations.

    if args.is_client() {
        println!("Running as client");
        let _client = Client::new();
    } else {
        client();
    };
    println!("end of program!");

    Ok(())
}
