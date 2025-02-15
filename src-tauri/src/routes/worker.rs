use maud::html;
use tauri::{command, State};
use tokio::sync::Mutex;

use crate::models::app_state::AppState;

#[command(async)]
pub async fn list_workers(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let server = state.lock().await;
    let workers = server.worker_db.read().await;
    match &workers.list_worker().await {
        Ok(data) => Ok(html! {
            @for worker in data {
                div key=(worker.spec.host) tauri-invoke="" hx-info="" hx-target="#workplace" {
                    table {
                        tbody {
                            tr {
                                td style="width:100%" {
                                    div { (worker.spec.host) }
                                    div { (worker.spec.os) " | " (worker.spec.arch) }
                                }
                            }
                        }
                    }
                }
            }
        }
        .0),
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
    let app_state = state.lock().await;
    let workers = app_state.worker_db.read().await;
    let content = match workers.get_worker(id).await {
        Some(worker) => html! {
            div {
                h1 { (format!("Computer: {}", worker.machine_id)) };
                h3 { "Hardware Info:" };
                p { (format!("System: {} | {}", worker.spec.os, worker.spec.arch))}
                p { (format!("CPU: {} | ({} threads)", worker.spec.cpu, worker.spec.cores)) };
                p { (format!("Ram: {} GB", worker.spec.memory / ( 1024 * 1024 )))}
                @if let Some(gpu) = worker.spec.gpu {
                    p { (format!("GPU: {}", gpu)) };
                } @else {
                    p { "GPU: N/A" };
                };

                // display current task below.
            };
        }
        .into_string(),
        None => return Ok("".to_owned()),
    };
    Ok(content)
}
