use crate::services::tauri_app::UiCommand;
use futures::channel::mpsc::Sender;

#[derive(Clone)]
pub struct AppState {
    pub invoke: Sender<UiCommand>,
}
