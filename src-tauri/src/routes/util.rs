use tauri::{command, AppHandle};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_fs::FilePath;

#[command(async)]
pub async fn select_directory(app: AppHandle) -> Result<String, String> {
    match app.dialog().file().blocking_pick_file() {
        Some(file_path) => {
            
            Ok(match file_path {
                FilePath::Path(path) => path.to_str().unwrap().to_string(),
                FilePath::Url(uri) => uri.to_string()
            })
        },
        None => Err("".to_owned())
    }
}