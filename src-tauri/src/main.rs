// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// use local_ip_address::local_ip;
// use std::{env,
//     fs::File,
//     io::{self, Read},
//     net::{IpAddr, TcpListener, TcpStream}, };

use std::env;

use blender::Blender;

pub mod blender;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// from the node we can reference to?

// globabally
// fn client() {
//     tauri::Builder::default()
//         .setup(|app| {
//             // now that we know what the app version is - we can use it to set our global version variable, as our main node reference.
//             println!("{}", app.package_info().version);
//             Ok(())
//         })
//         .invoke_handler(tauri::generate_handler![greet])
//         .run(tauri::generate_context!())
//         .expect("error while running tauri application");
// }

// as a server role, you are responsible to do the following requirement:
// open TcpListener and await for connection from client
// Once connection received, with information containing blender file and render settings
// check and see if blender exist
//      download and install matching blender configuration from the blender settings

// fn server(server_settings: &ServerSettings) {
//     let ip = local_ip().unwrap();

//     println!(
//         "IP Addresses of this Server:{:?}:{}",
//         ip, &server_settings.port
//     );

//     println!("Cleaing up old session...");
//     cleanup_old_sessions();

//     // let listener = TcpListener::bind(("0.0.0.0", server_settings.port))
//     //     .expect("Could not bind! Could there be a port in use?");
//     // for stream in listener.incoming() {
//     //     match stream {
//     //         Err(e) => {
//     //             eprintln!("Failed: {}", e)
//     //         }
//     //         Ok(stream) => spawn(|| {
//     //             println!("{}", handle_client(stream));
//     //         }),
//     //     }
//     // }
// }

// fn handle_client(mut stream: TcpStream) -> io::Result<()> {
//     println!("New client {}", stream.peer_name());
//     let mut buf = [0u8; 4096];
//     loop {
//         if stream.read(&mut buf) is Ok(data) {
//             stream.write(buf.slice(0, data));
//         } else {
//             break;
//         }
//     }
//     Ok(())
// }

fn main() -> std::io::Result<()> {
    // parse argument input here

    // obtain configurations

    // cleanup old sessions ?

    // initialize service listener
    // server(&settings);

    // for this month, I want to focus on having the ability to launch blender and send a render job, if possible
    // here we will ask for the user's blender file - we will use the scene file as a rendering present. Do not worry about gpu/cpu stuff. Just make this work.

    let mut path = env::current_dir()?;
    path.push("test.blend");

    let output = env::current_dir()?;
    let blend = Blender::default();
    Blender::render(&blend, path, output, 1);
    Ok(())
}
