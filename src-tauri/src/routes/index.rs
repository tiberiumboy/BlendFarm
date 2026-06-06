use maud::{PreEscaped, html};
use tauri::{State, command};
use tokio::sync::Mutex;
use crate::constant::WORKPLACE;
use crate::models::app_state::AppState;
use crate::routes::job::{render_list_job, render_job_detail_page};

// separate this?
#[command(async)]
pub async fn index(state: State<'_,Mutex<AppState>>) -> Result<String, String> {
    // Design to load content and page for the index.
    let mut app_state = state.lock().await;
    let jobs = app_state.list_jobs().await.map_err(|e| e.to_string())?;
    let list_job_render = render_list_job(&jobs);
    let job_detail = match &jobs {
        Some(job_list) => {
            match job_list.first() {
                Some(job) => app_state.fetch_job(job.id.clone()).await,
                None => None
            }
        },
        None => None 
    };
    let front_page_render = render_job_detail_page(&job_detail);
    
    Ok(html! (
        div {
            div class="sidebar" {
                nav {
                    ul class="nav-menu-items" {
                        
                        // li key="manager" class="nav-bar" tauri-invoke="remote_render_page" hx-target=(format!("#{WORKPLACE}")) {
                        //     span { "Remote Render" }
                        // };

                        li key="setting" class="nav-bar" tauri-invoke="setting_page" hx-target=(format!("#{WORKPLACE}")) {
                            span { "Setting" }
                        };
                    };
                };
                div {
                    h3 { "Jobs" }

                    button tauri-invoke="open_dialog_for_blend_file" hx-target="body" hx-swap="beforeend" {
                        "Import"
                    };

                    div class="group" id="joblist" {
                        (PreEscaped(list_job_render));
                    };
                }

                // div {
                //     h2 { "Computer Nodes" };
                //     // hx-trigger="every 10s" - omitting this as this was spamming console log
                //     div class="group" id="workers" tauri-invoke="list_workers" hx-target="this" {};
                // };
            };

        }
        main id=(WORKPLACE) {
            (PreEscaped(front_page_render))
        };
    ).0)
}