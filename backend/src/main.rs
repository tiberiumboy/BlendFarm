// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::controllers::remote_render::{
    create_job, create_node, delete_job, delete_node, delete_project, edit_node, import_project,
    list_job, list_node, list_projects, sync_project,
};

use crate::controllers::settings::{add_blender_installation, list_blender_installation};
use crate::models::{data::Data, render_node::RenderNode};
use blender::blender::Blender;
use blender::{args::Args, mode::Mode};
use semver::Version;
// use services::multicast::multicast;
// use services::receiver::receive;
use models::server_setting::ServerSetting;
use std::path::{Path, PathBuf};
use std::{env, io::Result, sync::Mutex /* thread */};
use tauri::generate_handler;

pub mod controllers;
pub mod models;
pub mod services;

// globabally
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
            // settings
            add_blender_installation,
            list_blender_installation,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[allow(dead_code)]
fn test_reading_blender_files(file: impl AsRef<Path>, version: &Version) -> Result<()> {
    let mut server_settings = ServerSetting::load();
    // eventually we would want to check if the version is already installed on the machine.
    // otherwise download and install the version prior to run this script.
    let blender = match server_settings
        .blenders
        .iter()
        .find(|&x| &x.version == version)
    {
        Some(blender) => blender.to_owned(),
        None => {
            let blender = Blender::download(version, &server_settings.blender_data).unwrap();
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
    let version = Version::new(4, 1, 0);
    let file = match env::consts::OS {
        "macos" => PathBuf::from("/Users/Shared/triangle.blend"),
        "linux" => PathBuf::from("/home/jordan/Downloads/fire_fx.blend"),
        _ => todo!(),
    };
    let _ = test_reading_blender_files(&file, &version);

    client();

    Ok(())
}
