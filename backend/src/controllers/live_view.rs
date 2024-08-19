use std::fs::File;

/*
    The idea behind this is to allow a scene you're working on to refresh and render from remote computer parts of the viewport render.
    Almost like linescan rendering.

    TODO: Find a way to pipe render preview from Blender's .so/.a/.dll?
    TODO: Find a way to receive and send data across network
*/

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
