use crate::models::project_file::ProjectFile;
use tauri::api::dialog::FileDialogBuilder;

// pub struct ProjectContext<'a> {
//     pub col: &'a mut Vec<ProjectFile>,
// }

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
// in this case here, this is where I would setup configuration and start mapping things out?
// question is, how do I access objects? E.g. If I want to update server settings
// or send command from a specific node?
#[tauri::command]
pub fn add_project() {
    // app crashed when api block thread. Do not use tauri::api::dialog::blocking::* apis.
    // could we not access tauri api side from react for filedialogbuilder?
    FileDialogBuilder::new().pick_file(|path| match path {
        Some(file_path) => {
            let project_file = ProjectFile::create(file_path);
            let msg = serde_json::to_string(&project_file).unwrap();
            println!("{msg}");
        }
        None => {
            println!("Operatin aborted - user exit the dialog");
        }
    });
    // can we have some sort of mechanism to hold data collection as long as this program is alive?
    // something we can append this list to the collections and reveal?
}

#[tauri::command]
pub fn edit_project() {}

// fn load_blend_file(name: &str) -> String {
//     name.to_owned()
// }

#[tauri::command]
pub fn load_project_list() {
    // generate a list of ProjectList to show on the forum
}
