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
use async_std::fs;
use blender::manager::Manager as BlenderManager;
use clap::{Parser, Subcommand};
use dotenv::dotenv;
use models::network;
use models::{app_state::AppState /* server_setting::ServerSetting */};
use services::data_store::sqlite_job_store::SqliteJobStore;
use services::data_store::sqlite_task_store::SqliteTaskStore;
use services::data_store::sqlite_worker_store::SqliteWorkerStore;
use services::{blend_farm::BlendFarm, cli_app::CliApp, tauri_app::TauriApp};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use std::sync::Arc;
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

async fn config_sqlite_db() -> Result<SqlitePool, sqlx::Error> {
    let mut path = BlenderManager::get_config_dir();
    path = path.join("blendfarm.db");

    // create file if it doesn't exist (.config/BlendFarm/blendfarm.db)
    let _ = fs::File::create(&path).await;

    // TODO: Consider thinking about the design behind this. Should we store database connection here or somewhere else?
    let url = format!("sqlite://{}", path.as_os_str().to_str().unwrap());
    // macos: "sqlite:///Users/megamind/Library/Application Support/BlendFarm/blendfarm.db"
    // dbg!(&url);
    let pool = SqlitePoolOptions::new().connect(&url).await?;
    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    dotenv().ok();
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    // to run custom behaviour
    let cli = Cli::parse();

    let db = config_sqlite_db()
        .await
        .expect("Must have database connection!");

    // must have working network services
    let (service, controller, receiver) =
        network::new().await.expect("Fail to start network service");

    // start network service async
    spawn(service.run());

    let _ = match cli.command {
        // run as client mode.
        Some(Commands::Client) => {
            // could this be reconsidered?
            let task_store = SqliteTaskStore::new(db.clone());
            let task_store = Arc::new(RwLock::new(task_store));
            CliApp::new(task_store)
                .run(controller, receiver)
                .await
                .map_err(|e| println!("Error running Cli app: {e:?}"))
        }

        // run as GUI mode.
        _ => {
            let job_store = SqliteJobStore::new(db.clone());
            let worker_store = SqliteWorkerStore::new(db.clone());

            let job_store = Arc::new(RwLock::new(job_store));
            let worker_store = Arc::new(RwLock::new(worker_store));
            TauriApp::new(worker_store, job_store)
                .await
                .run(controller, receiver)
                .await
                .map_err(|e| eprintln!("Fail to run Tauri app! {e:?}"))
        }
    };
}
