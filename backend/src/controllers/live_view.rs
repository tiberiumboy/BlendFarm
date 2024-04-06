use std::fs::File;

pub struct LiveView {
    file: File,
}

#[tauri::command]
pub fn load_file(_app: tauri::AppHandle) {
    // load the project file
    // spin up render_node to send the files over
    // then have it prepare to render section of it
    // and return the result to this view
}
