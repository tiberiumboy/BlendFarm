/*
    Developer blog:
    - Had a brain fart trying to figure out some ideas allowing me to run this application as either client or server
        Originally thought of using Clap library to parse in input, but when I run `cargo tauri dev -- test` the application fail to compile due to unknown arguments when running web framework?
        TODO: How can I pass in argument for this application executable?
    - Had an idea that allows user remotely to locally add blender installation without using GUI interface,
        This would serves two purposes - allow user to expressly select which blender version they can choose from the remote machine and
        prevent multiple download instances for the node, in case the target machine does not have it pre-installed.
    - Eventually, I will need to find a way to spin up a virtual machine and run blender farm on that machine to see about getting networking protocol working in place.
        This will allow me to do two things - I can continue to develop without needing to fire up a remote machine to test this and
        verify all packet works as intended while I can run the code in parallel to see if there's any issue I need to work overhead.
        This might be another big project to work over the summer to understand how network works in Rust.
*/

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::controllers::remote_render::{
    create_job, create_node, delete_job, delete_node, delete_project, edit_node, import_project,
    list_job, list_node, list_projects, list_versions,
};
use crate::controllers::settings::{
    add_blender_installation, get_server_settings, list_blender_installation,
    remove_blender_installation,
};
use crate::models::{data::Data, render_node::RenderNode, server::Server};
use blender::blender::Blender;
use blender::{args::Args, mode::Mode};
use gethostname::gethostname;
use models::{client::Client, server_setting::ServerSetting};
use semver::Version;
use std::path::{Path, PathBuf};
use std::thread;
use std::{env, io::Result, sync::Mutex};
use tauri::generate_handler;

pub mod controllers;
pub mod models;
pub mod services;

// when the app starts up, I would need to have access to onfigs. Config is loaded from json file - which can be access by user or program - it must be validate first before anything,
// I will have to create a manager struct -this is self managed by user action. e.g. new node, edit project files, delete jobs, etc.
fn client(server_setting: &ServerSetting) {
    println!("About to run server!");

    // is there a clear and better way to get around this?
    // I do not want to have any dangling threads if we have to run async
    let mut server = Server::new(server_setting.port).expect("Failed to create server");

    // Find a way to hold reference to this struct, and keep that struct as long lived until server produce an error or the app shutdown
    // let server = Arc::new(Mutex::new(server));
    // let network_handle = NamespacedThread::spawn("Why do I need a name for this?", move || {
    thread::spawn(move || {
        server.run();
    });
    // });

    println!("Successfully initialize the server");

    // panic!("This will never call until server finishes!");

    let mut data = Data::default();
    // I would like to find a better way to update or append data to render_nodes,
    // but I need to review more context about handling context like this in rust.
    // I understand Mutex, but I do not know if it's any safe to create pointers inside data struct from mutex memory.
    // "Do not communicate with shared memory"
    let localhost = RenderNode::default();
    data.render_nodes.push(localhost);

    let ctx = Mutex::new(data);

    // why I can't dive into implementation details here?
    tauri::Builder::default()
        // https://docs.rs/tauri/1.6.8/tauri/struct.Builder.html#method.manage
        // It is indeed possible to have more than one manage - which I will be taking advantage over how I can share and mutate configuration data across this platform.
        .manage(ctx)
        // .setup(|app| {
        //     // now that we know what the app version is - we can use it to set our global version variable, as our main node reference.
        //     println!("{}", app.package_info().version);
        //     Ok(())
        // })
        .invoke_handler(generate_handler![
            import_project,
            // sync_project,
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
            // settings
            add_blender_installation,
            list_blender_installation,
            remove_blender_installation,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// this code is only used to test and download blender version from the internet.
#[allow(dead_code)]
fn test_downloading_blender(version: Version) {
    // fetch the server settings to identify where we can save blender installation to.
    let server_setting = ServerSetting::load();
    let blender = Blender::download(version, server_setting.blender_dir);
    // verify that blender returned ok, otherwise fail if we have issues (internet connection/permission issue?)
    assert!(blender.is_ok());
}

/// This code is only used to test out downloading blender from source or reuse existing installation of blender, render a test scene example, and output the result.
#[allow(dead_code)]
fn test_reading_blender_files(file: impl AsRef<Path>, version: Version) {
    let mut server_settings = ServerSetting::load();

    // eventually we would want to check if the version is already installed on the machine.
    // otherwise download and install the version prior to run this script.
    let blender = server_settings.get_blender(version);

    // This part of the code is used to test and verify that we can successfully run blender
    let output = file.as_ref().parent().unwrap();
    let args = Args::new(file.as_ref(), PathBuf::from(output), Mode::Frame(1));
    let render_path = blender.render(&args);
    assert!(render_path.is_ok());
}

fn run_as_node(port: u16) {
    let hostname = gethostname().into_string().unwrap();
    match Client::new(&hostname, port) {
        Ok(client) => client.run(),
        Err(err) => println!("Cannot run the client! {}", err),
    }
}

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<String>>();
    // get the machine configuration here, and cache the result for poll request
    // we're making the assumption that the device card is available and ready when this app launches
    // obtain configurations

    // initialize service listener
    // here we will ask for the user's blender file

    // now that we have a unit test to cover whether we can actually run blender from the desire machine, we should now
    // work on getting network stuff working together! yay!
    // Assuming this code was compiled and run from ./backend dir
    // let _ = test_reading_blender_files(PathBuf::from("./test.blend"), Version::new(4, 1, 0));

    // TODO: It would be nice to include command line utility to let the user add blender installation from remotely.
    // TODO: consider looking into clap?
    // TOOD: If I build this application, how can I invoke commands directly? Do more search and test to see if there's a way for me to allow run this code if possible without having to separate the apps.
    // The command line would take an argument of --add or -a to append local blender installation from the local machine to the configurations.
    // Just to run some test here - run as "cargo run -- test"
    let server_setting = ServerSetting::load();

    if args.contains(&"test".to_owned()) {
        run_as_node(server_setting.port);
    } else {
        client(&server_setting);
    }

    Ok(())
}
