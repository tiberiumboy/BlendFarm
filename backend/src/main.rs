// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::controllers::remote_render::{
    create_job, create_node, delete_job, delete_node, edit_job, edit_node, list_job, list_node,
};
use crate::models::{data::Data, project_file::ProjectFile};
// use services::receiver::receive;
use crate::services::blender::Blender;
use std::path::PathBuf;
use std::{env, io::Result /* , thread*/, sync::Mutex};
use tauri::generate_handler;

pub mod controllers;
pub mod models;
pub mod services;

// globabally
#[allow(dead_code)]
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

// eventually, I want to get to a point where I could use blender to render an image or return an error.
// it would be nice to provide some kind of user interface to keep user entertained on the GUI side - e.g. percentage?
fn test_render() -> Result<()> {
    // load blend file. A simple scene with cube and plane. Ideally used for debugging purposes only.
    let mut path = env::current_dir()?;
    path.push("test");
    path.set_extension("blend");
    let project = ProjectFile::new(&path);

    // linux
    let path = PathBuf::from("/home/jordan/Downloads/blender/blender");
    // macOS
    // let path = PathBuf::from("/Applications/Blender.app/Contents/MacOS/Blender");

    // here we reference blender from given test path.
    let mut blender = Blender::from_executable(path).unwrap();

    // I now call render to invoke blender - returns file path of rendered output.
    let output = blender.render(&project, 1).unwrap();

    // let's see what the output does for now.
    dbg!(&output);

    Ok(())
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
    //
    // here we will ask for the user's blender file

    // now that we have a unit test to cover whether we can actually run blender from the desire machine, we should now
    // work on getting network stuff working together! yay!
    let _ = test_render();

    // client();

    Ok(())
}

// /home/jordan/Documents/src/rust/BlendFarm/backend/test.blend
