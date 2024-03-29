use crate::models::{data::Data, project_file::ProjectFile};
use std::sync::Mutex;
use tauri::api::dialog::FileDialogBuilder;
use tauri::{command, Manager};

// pub fn project() -> FnOnce<T> {
//     generate_handler![add_project, edit_project, load_project_list]
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
    let dlg = FileDialogBuilder::new();
    let result = dlg.pick_files(move |f| match f {
        Some()
    })
    //     move |path| match path {
    //     Some(file_path) => {
    //         let project_file = ProjectFile::new(&file_path);
    //         let ctx_mutex = app.state::<Mutex<Data>>();
    //         let mut ctx = ctx_mutex.lock().unwrap();
    //         ctx.project_files.push(project_file);
    //         println!("{:?}", ctx);
    //     }
    //     None => {
    //         println!("Operatin aborted - user exit the dialog");
    //     }
    // });
    // can we have some sort of mechanism to hold data collection as long as this program is alive?
    // something we can append this list to the collections and reveal?
}

#[command]
pub fn edit_project() {}

// fn load_blend_file(name: &str) -> String {
//     name.to_owned()
// }

#[command]
pub fn load_project_list(app: tauri::AppHandle) -> String {
    // generate a list of ProjectList to show on the forum
    let ctx_mutex = app.state::<Mutex<Data>>();
    let ctx = ctx_mutex.lock().unwrap();
    let data = serde_json::to_string(&ctx.project_files).unwrap();
    // println!("{data}");
    // let _ = app.emit_all("project_list", data);
    data
}
