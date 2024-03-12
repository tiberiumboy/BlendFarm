// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::controllers::{
    connection::{create_node, delete_node, edit_node, list_node},
    project::{add_project, edit_project, load_project_list},
};
use crate::models::context::Context;
use message_io::{
    network::{NetEvent, Transport},
    node,
};
use std::{env, sync::Mutex, thread};
use tauri::generate_handler;

pub mod controllers;
pub mod models;
pub mod services;

// globabally
fn client() {
    let ctx = Mutex::new(Context::default());
    tauri::Builder::default()
        .manage(ctx)
        // .setup(|app| {
        //     // now that we know what the app version is - we can use it to set our global version variable, as our main node reference.
        //     println!("{}", app.package_info().version);
        //     Ok(())
        // })
        // Hmm find a way to load multiple of handlers? from different page source?
        // I feel like there should be a better way to manage this?
        .invoke_handler(generate_handler![
            add_project,
            edit_project,
            load_project_list,
            create_node,
            list_node,
            edit_node,
            delete_node
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// as a server role, you are responsible to do the following requirement:
// open TcpListener and await for connection from client
// Once connection received, with information containing blender file and render settings
// check and see if blender exist
//      download and install matching blender configuration from the blender settings

fn setup_listeners() {
    let (handler, listener) = node::split::<()>();

    handler
        .network()
        .listen(Transport::FramedTcp, "localhost:15000")
        .unwrap();

    listener.for_each(move |event| match event.network() {
        NetEvent::Connected(_, _) => unreachable!(),
        NetEvent::Accepted(endpoint, _listener) => {
            println!("Client connected {}", endpoint.addr().ip());
        }
        NetEvent::Message(endpoint, data) => {
            println!("Received: {}", String::from_utf8_lossy(data));
            handler.network().send(endpoint, data);
        }
        NetEvent::Disconnected(endpoint) => {
            println!("Disconnected {}", endpoint.addr().ip());
        }
    });
}

fn main() -> std::io::Result<()> {
    // get the machine configuration here, and cache the result for poll request
    // we're making the assumption that the device card is available and ready when this app launches

    // let input: &str = "indivisibility";

    // let chars = input.chars().collect();
    // char

    // parse argument input here
    // let args = std::env::args();
    // println!("{args:?}");
    // let config = Config::load(); // Config::load();
    // let args = std::env::args();
    // println!("{args:?}");
    // let config = Config::load(); // Config::load();

    // obtain configurations

    // cleanup old sessions ? Assuming temp file(s) get deleted anyway - should we need to worry about this?
    // we could use this as an opportunity to clean cache files in case a client request it?
    // could provide computer spec info? - feature request?
    // println!("Cleaing up old session...");
    // cleanup_old_sessions();

    // initialize service listener
    thread::spawn(|| {
        setup_listeners();
        // let settings = ServerSettings::default();
        // server(&settings);
    });

    // for this month, I want to focus on having the ability to send a render job,
    // I can render now! Mac is special
    // here we will ask for the user's blender file - we will use the scene file as a rendering present. Do not worry about gpu/cpu stuff. Just make this work.

    // let mut path = env::current_dir()?;
    // path.push("test.blend");

    // let output = env::current_dir()?;
    // let blend = Blender::default();
    // match Blender::render(&blend, path, output, 1) {
    //     Ok(result) => println!("{result:?}"),
    //   https://www.youtube.com/watch?v=zlthUnIW7wI  Err(e) => println!("Failed to render: {e:?}"),
    // };

    client();

    Ok(())
}
