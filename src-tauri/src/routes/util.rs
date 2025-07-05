use tauri::{command, AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_fs::FilePath;
use tokio::sync::Mutex;

#[command(async)]
pub async fn select_directory(state: State<'_, Mutex<AppHandle>>) -> Result<String, String> {
    let app = state.lock().await;
    match app.dialog().file().blocking_pick_folder() {
        Some(file_path) => Ok(match file_path {
            FilePath::Path(path) => path.to_str().unwrap().to_string(),
            FilePath::Url(uri) => uri.to_string(),
        }),
        None => Err("".to_owned()),
    }
}

#[command(async)]
pub async fn select_file(state: State<'_, Mutex<AppHandle>>) -> Result<String, ()> {
    let app = state.lock().await;
    match app.dialog().file().blocking_pick_file() {
        Some(file_path) => Ok(match file_path {
            FilePath::Path(path) => path.to_str().unwrap().to_string(),
            FilePath::Url(uri) => uri.to_string(),
        }),
        None => Err(()),
    }
}

#[command]
pub fn open_path(path: &str) {
    println!("Trying to open {path}");
}
