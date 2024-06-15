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
use crate::models::{data::Data, render_node::RenderNode};
use blender::blender::Blender;
use blender::{args::Args, mode::Mode};
use semver::Version;
use services::multicast::multicast;
// use services::receiver::receive;
use models::server_setting::ServerSetting;
use std::path::{Path, PathBuf};
use std::{env, io::Result, sync::Mutex};
use tauri::generate_handler;

pub mod controllers;
pub mod models;
pub mod services;

// globabally
#[allow(dead_code)]
fn client() {
    let localhost = RenderNode::create_localhost();
    let mut data = Data::default();
    data.render_nodes.push(localhost);
    let ctx = Mutex::new(data);

    tauri::Builder::default()
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

#[allow(dead_code)]
fn test_downloading_blender() -> Result<Blender> {
    let server_setting = ServerSetting::load();
    let version = Version::new(4, 1, 0);
    let blender = Blender::download(version, server_setting.blender_dir).unwrap();
    dbg!(&blender);
    Ok(blender)
}

/// This code is only used to test out downloading blender from source or reuse existing installation of blender, render a test scene example, and output the result.
#[allow(dead_code)]
fn test_reading_blender_files(file: impl AsRef<Path>, version: Version) -> Result<()> {
    let mut server_settings = ServerSetting::load();

    // eventually we would want to check if the version is already installed on the machine.
    // otherwise download and install the version prior to run this script.
    let blender = match server_settings
        .blenders
        .iter()
        .find(|&x| x.version == version)
    {
        Some(blender) => blender.to_owned(),
        None => {
            let blender = Blender::download(version, &server_settings.blender_dir).unwrap();
            server_settings.blenders.push(blender.clone());
            server_settings.save();
            blender
        }
    };

    // This part of the code is used to test and verify that we can successfully run blender
    let output = file.as_ref().parent().unwrap();
    let args = Args::new(file.as_ref(), PathBuf::from(output), Mode::Frame(1));
    let render_path = blender.render(&args).unwrap();
    dbg!(render_path);
    Ok(())
}

fn main() -> Result<()> {
    // get the machine configuration here, and cache the result for poll request
    // we're making the assumption that the device card is available and ready when this app launches
    // obtain configurations

    // initialize service listener
    // thread::spawn(|| {
    // receive();
    multicast();
    // });
    //
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
    // if args.contains(&"test".to_owned()) {
    // } else {
    // client();
    // }

    Ok(())
}
