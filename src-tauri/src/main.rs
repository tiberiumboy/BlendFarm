// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use local_ip_address::local_ip;
use message_io::{
    network::{NetEvent, Transport},
    node,
};
use page::project::{add_project, edit_project, load_project_list};
use server_settings::ServerSettings;
use std::{
    env,
    io::Read,
    net::{TcpListener, TcpStream},
    // result,
    thread,
    // time::Duration,
};

pub mod context;
pub mod page;
pub mod render_client;
pub mod server_settings;

// from the node we can reference to?

// globabally
fn client() {
    tauri::Builder::default()
        .setup(|app| {
            // now that we know what the app version is - we can use it to set our global version variable, as our main node reference.
            println!("{}", app.package_info().version);
            Ok(())
        })
        // Hmm find a way to load multiple of handlers? from different page source?
        // I feel like there should be a better way to manage this?
        .invoke_handler(tauri::generate_handler![
            add_project,
            edit_project,
            load_project_list
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// as a server role, you are responsible to do the following requirement:
// open TcpListener and await for connection from client
// Once connection received, with information containing blender file and render settings
// check and see if blender exist
//      download and install matching blender configuration from the blender settings

fn server(server_settings: &ServerSettings) {
    let ip = local_ip().unwrap();

    let listener = TcpListener::bind((ip, server_settings.port))
        .expect("Could not bind! Could there be a port in use?");

    println!(
        "IP Addresses of this Server:{:?}",
        listener.local_addr().unwrap()
    );

    // the pwroblem with this is that I need this node to be locked to one host/server.
    // if the host/server connects to this, this node needs to respect that.
    // currently accepts any and all connections whatsoever.
    for result in listener.incoming() {
        match result {
            Err(e) => {
                eprintln!("Failed: {}", e)
            }
            // need to check the documentation on this one - I don't know if we're blocking main thread or this is running in parallel?
            Ok(stream) => thread::scope(|_| {
                println!("{:?}", handle_client(stream));
            }),
        }
    }
}

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    if let Ok(adr) = stream.peer_addr() {
        println!("New client {:?}", adr);
    }
    let mut buf = [0u8; 4096];
    loop {
        if let Ok(_) = stream.read(&mut buf) {
            // here we write back the response?
            // let _ = stream.write(b"Received!"); // returns the length writing back?
        } else {
            break;
        }
    }
    Ok(())
}

fn setup_listeners() {
    let (handler, listener) = node::split::<()>();

    handler
        .network()
        .listen(Transport::FramedTcp, "0.0.0.0:15000")
        .unwrap();
    handler
        .network()
        .listen(Transport::Udp, "0.0.0.0:15001")
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
            println!("Client disconnected {}", endpoint.addr().ip())
        }
    });
}

fn main() -> std::io::Result<()> {
    // get the machine configuration here, and cache the result for poll request
    // we're making the assumption that the device card is available and ready when this app launches

    // parse argument input here
    // let args = std::env::args();
    // println!("{args:?}");
    // let config = Config::load(); // Config::load();

    // obtain configurations

    // cleanup old sessions ? Assuming temp file(s) get deleted anyway - should we need to worry about this?
    // we could use this as an opportunity to clean cache files in case a client request it?
    // could provide computer spec info? - feature request?
    // println!("Cleaing up old session...");
    // cleanup_old_sessions();

    setup_listeners();
    // initialize service listener
    // thread::spawn(|| {
    //     let settings = ServerSettings::default();
    //     server(&settings);
    // });

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
