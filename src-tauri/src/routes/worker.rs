use maud::html;
use tauri::{command, State};
use tokio::sync::Mutex;

use crate::models::app_state::AppState;

#[command(async)]
pub async fn list_workers(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let server = state.lock().await;
    let workers = server.worker_db.read().await;
    match &workers.list_worker().await {
        Ok(data) => Ok(serde_json::to_string(data).unwrap()),
        Err(e) => Err(e.to_string()),
    }
}

/*
<div>
            <h1>Computer: {node.spec?.host}</h1>
            <h3>Specs</h3>
            <p>CPU: {node.spec?.cpu}</p>
            <p>Ram: { (node.spec?.memory ?? 0 ) / ( 1024 * 1024 )} Gb</p>
            <p>OS: {node.spec?.os} | {node.spec?.arch}</p>
            {/* how can I make a if condition to display GPU if it's available? */}
            <p>GPU: {node.spec?.gpu}</p>
            
            <h3>Current Task:</h3>
            <p>Task: None</p>
            <p>Frame: 0/0</p>

            <h3>Monitor</h3>
            {/* Draw a Linegraph to display CPU/Mem usage */}
        </div>

*/
#[command(async)]
pub async fn get_worker(state: State<'_, Mutex<AppState>>, id: String) -> Result<String, String> {
    let worker = "".to_owned();
    let content = html!{
        div {
            h1 { (format!("Computer: {}", 1)) };
            h3 { "Hardware Info:" };
            p { (format!("System: {} | {}", 1, 1))}
            p { (format!("CPU:{}", 1)) };
            p { (format!("Ram:{}", 1))}
            p { (format!("GPU:{}", 1))}

            // display current task below.
        };
    }.into_string();
    Ok(content)
}
