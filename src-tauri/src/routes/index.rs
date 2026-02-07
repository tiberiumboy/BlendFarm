use maud::html;
use tauri::command;
use crate::constant::WORKPLACE;

// separate this?
#[command]
pub fn index() -> String {
    html! (
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

                    // Is there a way to select the first item on the list by default?
                    // TODO: Take a look into hx-swap-oob on how we can refresh when a record is deleted or added
                    div class="group" id="joblist" tauri-invoke="list_jobs" hx-trigger="load" hx-target="this";
                }

                // div {
                //     h2 { "Computer Nodes" };
                //     // hx-trigger="every 10s" - omitting this as this was spamming console log
                //     div class="group" id="workers" tauri-invoke="list_workers" hx-target="this" {};
                // };
            };

        }
        main id=(WORKPLACE);
    ).0
}