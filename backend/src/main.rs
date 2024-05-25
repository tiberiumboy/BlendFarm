// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::controllers::remote_render::{
    create_job, create_node, delete_job, delete_node, delete_project, edit_node, import_project,
    list_job, list_node, list_projects, sync_project,
};
use crate::models::{data::Data, render_node::RenderNode};
use blender::blender::Blender;
use blender::{args::Args, mode::Mode};
use semver::Version;
// use services::multicast::multicast;
// use services::receiver::receive;
use models::server_setting::ServerSetting;
use std::path::PathBuf;
use std::{env, io::Result, sync::Mutex /* thread */};
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
            sync_project,
            create_node,
            create_job,
            delete_node,
            delete_project,
            delete_job,
            edit_node,
            list_node,
            list_projects,
            list_job,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[allow(dead_code)]
fn test_reading_blender_files() -> Result<()> {
    let version = Version::new(3, 0, 0);
    let server_settings = ServerSetting::load();
    // eventually we would want to check if the version is already installed on the machine.
    // otherwise download and install the version prior to run this script.
    // For now - Let's go ahead and try download it just to make sure this is all working properly
    // let installed_blender = server_settings.blenders.find(|x| x.version == version);

    let installation_path = server_settings.blender_data;
    let blender = Blender::download(version, installation_path).unwrap();
    let args = Args::new(
        PathBuf::from("/home/jordan/Downloads/fire_fx.blend"),
        PathBuf::from("/home/jordan/Downloads/test.png"),
        Mode::Frame(1),
    );
    let render_path = blender.render(&args).unwrap();
    dbg!(render_path);
    Ok(())
}

fn main() -> Result<()> {
    // get the machine configuration here, and cache the result for poll request
    // we're making the assumption that the device card is available and ready when this app launches

    // parse argument input here
    // let args = std::env::args();
    // println!("{args:?}");
    // obtain configurations

    // initialize service listener
    // thread::spawn(|| {
    //     // receive();
    //     multicast();
    // });
    //
    // here we will ask for the user's blender file

    // now that we have a unit test to cover whether we can actually run blender from the desire machine, we should now
    // work on getting network stuff working together! yay!
    let _ = test_reading_blender_files();

    // client();

    Ok(())
}
