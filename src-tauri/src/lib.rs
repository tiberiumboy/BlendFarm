/*
Developer blog:
- Had a brain fart trying to figure out some ideas allowing me to run this application as either client or server
    Originally thought of using Clap library to parse in input, but when I run `cargo tauri dev -- test` the application fail to compile due to unknown arguments when running web framework?
    This issue has been solved by allowing certain argument to run. By default it will launch the manager version of this application.
    9/2/24
- Had an idea that allows user remotely to locally add blender installation without using GUI interface,
    This would serves two purposes - allow user to expressly select which blender version they can choose from the remote machine and
    prevent multiple download instances for the node, in case the target machine does not have it pre-installed.
- Eventually, I will need to find a way to spin up a virtual machine and run blender farm on that machine to see about getting networking protocol working in place.
    This will allow me to do two things - I can continue to develop without needing to fire up a remote machine to test this and
    verify all packet works as intended while I can run the code in parallel to see if there's any issue I need to work overhead.

[F] - find a way to allow GUI interface to run as client mode for non cli users.
[F] - consider using channel to stream data https://v2.tauri.app/develop/calling-frontend/#channels
[F] - Before release - find a way to add updater  https://v2.tauri.app/plugin/updater/
*/
// TODO: Create a miro diagram structure of how this application suppose to work
// Need a mapping to explain how network should perform over intranet
// Need a mapping to explain how blender manager is used and invoked for the job

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use blender::manager::Manager as BlenderManager;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use models::network;
use services::data_store::sqlite_task_store::SqliteTaskStore;
use services::{blend_farm::BlendFarm, cli_app::CliApp, tauri_app::TauriApp};
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::sync::Arc;
use tokio::spawn;
use tokio::sync::RwLock;

use crate::models::server_setting::ServerSetting;

pub mod domains;
pub mod models;
pub mod routes;
pub mod services;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Client,
}

async fn config_sqlite_db(file_name: &str) -> Result<SqlitePool, sqlx::Error> {
    // TODO: Ask for user preference.
    let user_pref = None;

    let path = BlenderManager::get_config_dir(user_pref).join(file_name);
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);
    SqlitePool::connect_with(options).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    dotenv().ok();

    // to collect user inputs for custom user preferences
    let cli = Cli::parse();
    
    // TODO: Ask Cli for the secret_key
    let secret_key = None;
    
    // same here, ask Cli before using default options
    let database_file_name = "blendfarm.db";
    
    // TODO: insist on loading user_pref here? if there's a custom cli command that insist user path for server settings, we would ask them there.
    let user_pref = ServerSetting::load();

    // initialize database connection
    let db: sqlx::Pool<sqlx::Sqlite> = config_sqlite_db(database_file_name)
        .await
        .expect("Must have database connection!");

    // must have working network services
    let (controller, receiver, mut server) = network::new(secret_key)
        .await
        .expect("Fail to start network service");

    // Network service is spun up on separate thread.
    spawn(async move {
        server.run().await;
    });


    let _ = match cli.command {
        // run as client mode.
        Some(Commands::Client) => {
            // eventually I'll move this code into it's own separate codeblock
            let task_store = SqliteTaskStore::new(db.clone());

            // we're sharing this across threads?
            let task_store = Arc::new(RwLock::new(task_store));

            // here the client wants database connection to task table. Why not provide database connection instead?
            CliApp::new(task_store)
                .run(controller, receiver)
                .await
                .map_err(|e| println!("Error running Cli app: {e:?}"))
        }

        // run as GUI mode.
        _ => TauriApp::new(&db)
            .await
            // we're clearing workers?
            .clear_workers_collection()
            .await
            .run(controller, receiver)
            .await
            .map_err(|e| eprintln!("Fail to run Tauri app! {e:?}")),
    };
}

#[cfg(test)]
mod test {
    use crate::config_sqlite_db;

    #[tokio::test]
    pub async fn validate_creating_database_structure() {
        let database_file_name = "blendfarm.db";
        let conn = config_sqlite_db(database_file_name).await;
        assert!(conn.is_ok());
    }
}
