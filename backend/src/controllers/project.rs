use std::sync::Mutex;

use crate::models::{data::Data, project_file::ProjectFile};
use tauri::api::dialog::FileDialogBuilder;
use tauri::{command, Manager};
// use thiserror::Error;

// pub fn project() -> FnOnce<T> {
//     generate_handler![add_project, edit_project, load_project_list]
// }

// #[derive(Error, Debug)]
// pub enum ProjectError {
//     #[error("Project not found")]
//     NotFound,
//     #[error("Project already exists")]
//     AlreadyExists,
// }

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
// in this case here, this is where I would setup configuration and start mapping things out?
// question is, how do I access objects? E.g. If I want to update server settings
// or send command from a specific node?
#[tauri::command]
pub fn add_project(app: tauri::AppHandle) {
    // app crashed when api block thread. Do not use tauri::api::dialog::blocking::* apis.
    // could we not access tauri api side from react for filedialogbuilder?
    // How can I block js from invoking next when I need to wait for this dialog to complete?
    // how can I only filter .blend extension format?
    FileDialogBuilder::new()
        .add_filter("Blender Files", &["blend"])
        .pick_files(move |path| match path {
            Some(file_paths) => {
                let ctx_mutex = app.state::<Mutex<Data>>();
                let mut ctx = ctx_mutex.lock().unwrap();
                for file_path in file_paths.iter() {
                    ctx.project_files.push(ProjectFile::new(file_path));
                }
            }
            None => {
                // do nothing
            }
        });
    // can we have some sort of mechanism to hold data collection as long as this program is alive?
    // something we can append this list to the collections and reveal?
}

// Delete project file from the list. if tmp is defined, delete that as well.
#[command]
pub fn delete_project(app: tauri::AppHandle, id: &str) {
    let ctx_mutex = app.state::<Mutex<Data>>();
    let mut ctx = ctx_mutex.lock().unwrap();
    let result = ctx.project_files.iter().find(|x| x.id == id);
    if let Some(project) = result {
        let _ = std::fs::remove_file(project.tmp.as_ref().unwrap());
    };
    ctx.project_files.retain(|x| x.id != id);
}

#[command]
pub fn load_project_list(app: tauri::AppHandle) -> String {
    // generate a list of ProjectList to show on the forum
    let ctx_mutex = app.state::<Mutex<Data>>();
    let ctx = ctx_mutex.lock().unwrap();
    serde_json::to_string(&ctx.project_files).unwrap()
    // let _ = app.emit_all("project_list", data);
}
