/*
Developer blog:
- Had a brain fart trying to figure out some ideas allowing me to run this application as either client or server
    Originally thought of using Clap library to parse in input, but when I run `cargo tauri dev -- test` the application fail to compile due to unknown arguments when running web framework?
    This issue has been solved by alllowing certain argument to run. By default it will try to launch the client user interface of the application.
    Additionally, I need to check into the argument and see if there's a way we could just allow user to run --server without ui interface?
    Interesting thoughts for sure
    9/2/24
- Decided to rely on using Tauri plugin for cli commands and subcommands. Use that instead of clap. Since Tauri already incorporates Clap anyway.
- Had an idea that allows user remotely to locally add blender installation without using GUI interface,
    This would serves two purposes - allow user to expressly select which blender version they can choose from the remote machine and
    prevent multiple download instances for the node, in case the target machine does not have it pre-installed.
- Eventually, I will need to find a way to spin up a virtual machine and run blender farm on that machine to see about getting networking protocol working in place.
    This will allow me to do two things - I can continue to develop without needing to fire up a remote machine to test this and
    verify all packet works as intended while I can run the code in parallel to see if there's any issue I need to work overhead.
    This might be another big project to work over the summer to understand how network works in Rust.

- I noticed that some of the function are getting called twice. Check and see what's going on with React UI side of things
    Research into profiling front end ui to ensure the app is not invoking the same command twice.

[F] - find a way to allow GUI interface to run as client mode for non cli users.
[F] - consider using channel to stream data https://v2.tauri.app/develop/calling-frontend/#channels
[F] - Before release - find a way to add updater  https://v2.tauri.app/plugin/updater/
*/
// TODO: Create a miro diagram structure of how this application suppose to work
// Need a mapping to explain how network should perform over intranet
// Need a mapping to explain how blender manager is used and invoked for the job
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use clap::{Parser, Subcommand};
use models::network;
use models::{app_state::AppState, server_setting::ServerSetting};
use services::data_store::surrealdb_task_store::SurrealDbTaskStore;
use services::{
    blend_farm::BlendFarm, cli_app::CliApp,
    data_store::surrealdb_worker_store::SurrealDbWorkerStore, tauri_app::TauriApp,
};
use std::sync::Arc;
use surrealdb::{
    engine::local::{Db, SurrealKv},
    Surreal,
};
use tokio::{spawn, sync::RwLock};
use tracing_subscriber::EnvFilter;

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

async fn config_surreal_db() -> Surreal<Db> {
    let db_path = ServerSetting::get_config_dir();
    let db = Surreal::new::<SurrealKv>(db_path)
        .await
        .expect("Fail to create database");
    db.use_ns("BlendFarm")
        .use_db("BlendFarm")
        .await
        .expect("Failed to specify namespace/database!");
    // make sure the schema is setup properly
    db.query(
        r#"
        DEFINE TABLE IF NOT EXISTS task SCHEMALESS;
        DEFINE FIELD IF NOT EXISTS peer_id TYPE 
    "#,
    )
    .await
    .expect("Should have permission to check for database schema");
    db
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    // to run custom behaviour
    let cli = Cli::parse();

    // create a database instance
    let db = config_surreal_db().await;
    let db = Arc::new(RwLock::new(db));
    let task_store = Arc::new(RwLock::new(SurrealDbTaskStore::new(db.clone())));
    // must have working network services
    let (service, controller, receiver) =
        network::new().await.expect("Fail to start network service");

    // start network service async
    spawn(service.run());

    if let Err(e) = match cli.command {
        // run as client mode.
        Some(Commands::Client) => CliApp::new(task_store).run(controller, receiver).await,
        // run as GUI mode.
        _ => TauriApp::new(db).run(controller, receiver).await,
    } {
        eprintln!("Something went terribly wrong? {e:?}");
    }
}
