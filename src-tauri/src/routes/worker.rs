use maud::html;
use serde_json::json;
use tauri::{command, State};
use tokio::sync::Mutex;

use crate::models::app_state::AppState;
use crate::services::tauri_app::WORKPLACE;

#[command(async)]
pub async fn list_workers(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let server = state.lock().await;
    let workers = server.worker_db.read().await;
    match &workers.list_worker().await {
        Ok(data) => {
            let content = match data.len() {
                0 => html! { div { } },
                _ => html! {
                    @for worker in data {
                        div {
                            table tauri-invoke="get_worker" hx-vals=(json!({ "machineId": worker.machine_id.to_base58() })) hx-target=(format!("#{WORKPLACE}")) {
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
                },
            };
            Ok(content.0)
        }
        Err(e) => {
            eprintln!("Received error on list workers: \n{e:?}");
            Ok(html!( div { }; ).0)
        }
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
pub async fn get_worker(state: State<'_, Mutex<AppState>>, machine_id: &str) -> Result<String, ()> {
    let app_state = state.lock().await;
    let workers = app_state.worker_db.read().await;
    match workers.get_worker(machine_id).await {
        Some(worker) => Ok(html! {
            div class="content" {
                h1 { (format!("Computer: {}", worker.spec.host)) };
                h3 { "Hardware Info:" };
                table {
                    tr {
                        th {
                            "System"
                        }
                        th {
                           "CPU"
                        }
                        th {
                            "Memory"
                        }
                        th {
                            "GPU"
                        }
                    }
                    tr {
                        td {
                            p { (worker.spec.os) }
                            span { (worker.spec.arch) }
                        }
                        td {
                            p { (worker.spec.cpu) }
                            span { (format!("({} cores)",worker.spec.cores)) }
                        }
                        td {
                            (format!("{}GB", worker.spec.memory / ( 1024 * 1024 * 1024 )))
                        }
                        td {
                            @if let Some(gpu) = worker.spec.gpu {
                                label { (gpu) };
                            } @else {
                                label { "N/A" };
                            };
                        }
                    }
                }

                h3 { "Task List" }
                table {
                    tr {
                        th {
                            "Project Name"
                        }
                        th {
                            "Progresss"
                        }
                    }
                    // TODO: Fill in the info from the worker machine here.
                }
            };
        }
        .0),
        None => Err(()),
    }
}
