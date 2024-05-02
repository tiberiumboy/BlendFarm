// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::controllers::remote_render::{
    create_job, create_node, delete_job, delete_node, delete_project, edit_node, import_project,
    list_job, list_node, list_projects, sync_project,
};
use crate::models::{data::Data, render_node::RenderNode};
use blender::args::Args;
use blender::blender::Blender;
use blender::mode::Mode;
// use services::multicast::multicast;
// use services::receiver::receive;

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

// eventually, I want to get to a point where I could use blender to render an image or return an error.
// it would be nice to provide some kind of user interface to keep user entertained on the GUI side - e.g. percentage?
fn test_render() -> Result<()> {
    // load blend file. A simple scene with cube and plane. Ideally used for debugging purposes only.
    let output = PathBuf::from("./backend/");
    let mut path = output.clone();
    path.push("test");
    path.set_extension("blend");

    let args = Args::new(path, output, Mode::Frame(1));

    // linux
    let path = match env::consts::OS {
        "linux" => PathBuf::from("/home/jordan/Downloads/blender/blender"),
        "macos" => PathBuf::from("/Applications/Blender.app/Contents/MacOS/Blender"),
        _ => panic!("unsupported OS"),
    };
    let mut blender = Blender::from_executable(path).unwrap();

    // I now call render to invoke blender - returns file path of rendered output.
    let _ = blender.render(&args).unwrap();

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
    // thread::spawn(|| {
    //     // receive();
    //     multicast();
    // });
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
