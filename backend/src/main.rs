// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::controllers::remote_render::{
    create_job, create_node, delete_job, delete_node, edit_job, edit_node, list_job, list_node,
};
use crate::models::{data::Data, project_file::ProjectFile};
use blender::args::Args;
use blender::blender::Blender;
use blender::mode::Mode;
use services::receiver::receive;

use regex::Regex;
use std::fs;
use std::path::PathBuf;
use std::{env, io::Result, sync::Mutex, thread};
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
#[allow(dead_code)]
fn test_render() -> Result<()> {
    // load blend file. A simple scene with cube and plane. Ideally used for debugging purposes only.
    let mut path = PathBuf::from("./backend/");
    let output = path.clone();
    path.push("test");
    path.set_extension("blend");

    // let project = ProjectFile::new(&path);
    let args = Args::new(path, output, Mode::Frame(1));

    // linux
    // let path = PathBuf::from("/home/jordan/Downloads/blender/blender");
    // macOS
    let path = PathBuf::from("/Applications/Blender.app/Contents/MacOS/Blender");
    let blender = Blender::from_executable(path).unwrap();

    // I now call render to invoke blender - returns file path of rendered output.
    let output = blender.render(&args).unwrap();

    // let's see what the output does for now.
    dbg!(&output);
    Ok(())
}

#[allow(dead_code)]
fn test_reading_blender_files() -> Result<()> {
    // will need to find a place for this.
    let re = Regex::new(r#"<a href="(?<url>.*?)">(?<name>.*?)</a>\s*(?<date>.*?)\s\s\s"#).unwrap();
    let content = fs::read_to_string("./src/examples/blender.net").unwrap();
    for (_, [url, name, date]) in re.captures_iter(&content).map(|c| c.extract()) {
        println!("url: {}, name: {}, date: {}", url, name, date);
    }

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
    thread::spawn(|| {
        receive();
    });
    //
    // here we will ask for the user's blender file

    // now that we have a unit test to cover whether we can actually run blender from the desire machine, we should now
    // work on getting network stuff working together! yay!
    // let _ = test_render();
    // let _ = test_reading_blender_files();

    client();

    Ok(())
}

// /home/jordan/Documents/src/rust/BlendFarm/backend/test.blend
