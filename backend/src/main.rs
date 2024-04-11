// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::controllers::remote_render::{
    create_job, create_node, delete_job, delete_node, edit_job, edit_node, list_job, list_node,
};
use crate::models::{data::Data, project_file::ProjectFile};
// use services::receiver::receive;
use crate::blender::version::Blender;
use std::path::PathBuf;
use std::{env, sync::Mutex /* , thread*/};
use tauri::generate_handler;

pub mod blender;
pub mod controllers;
pub mod models;
pub mod services;

// globabally
fn client() {
    let ctx = Mutex::new(Data::default());
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
            create_job,
            create_node,
            delete_job,
            delete_node,
            edit_job,
            edit_node,
            list_job,
            list_node,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() -> std::io::Result<()> {
    // get the machine configuration here, and cache the result for poll request
    // we're making the assumption that the device card is available and ready when this app launches

    // parse argument input here
    // let args = std::env::args();
    // println!("{args:?}");
    // obtain configurations

    // initialize service listener
    // thread::spawn(|| {
    //     receive();
    // });

    // for this month, I want to focus on having the ability to send a render job,
    // here we will ask for the user's blender file - we will use the scene file as a rendering present. Do not worry about gpu/cpu stuff. Just make this work.

    let mut path = env::current_dir()?;
    path.push("test.blend");

    let project = ProjectFile::new(&path);

    // can we assume that we have a default present loaded?
    // let path = PathBuf::from("~/Downloads/blender/blender");    // linux
    let path = PathBuf::from("/Applications/Blender.app/Contents/MacOS/Blender"); // macOS // why mac?
    let mut blender = Blender::from_executable(path).unwrap(); // TODO: handle unwrap

    // great this still works! fantastic!
    match blender.render(&project, 1) {
        Ok(result) => println!("{result:?}"),
        Err(e) => println!("{e:?}"),
    };

    // client();

    Ok(())
}

// /home/jordan/Documents/src/rust/BlendFarm/backend/test.blend
