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
- Ended up refactoring the program out. each struct have their respective files and folder associated with their group of services.
    I still have problem using libp2p. Originally had it working but it was locking up main thread and program from executing in async.
    Going to rely on example until I get this program working again.
[F] - find a way to allow GUI interface to run as client mode for non cli users.
[F] - consider using channel to stream data https://v2.tauri.app/develop/calling-frontend/#channels
[F] - Before release - find a way to add updater  https://v2.tauri.app/plugin/updater/
*/
// TODO: Create a miro diagram structure of how this application suppose to work
// Need a mapping to explain how network should perform over intranet
// Need a mapping to explain how blender manager is used and invoked for the job

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
// it might be interesting and useful if there's a debug mode enabled?
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use anyhow::Error;
use blender::manager::Manager as BlenderManager;
use blender::models::blender_config::BlenderConfig;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use libp2p::Multiaddr;
use services::{blend_farm::BlendFarm, cli_app::CliApp, tauri_app::TauriApp};
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::path::{Path, PathBuf};
use tokio::spawn;

use crate::constant::{JOB_TOPIC, NODE_TOPIC};
use crate::models::server_setting::ServerSetting;
use crate::network::controller::Controller;
use crate::services::app_context::AppContext;

pub mod constant;
pub mod domains;
pub mod models;
pub mod network;
pub mod routes;
pub mod services;

#[derive(Parser)]
struct Cli {
    #[arg(short, long, default_value=None)]
    config_path: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Client,
}

async fn config_sqlite_db(path: impl AsRef<Path>) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);
    SqlitePool::connect_with(options).await
}

async fn setup_connection(controller: &mut Controller) -> Result<(), Error> {
    // Listen on all interfaces and whatever port OS assigns
    let tcp: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse().expect("Shouldn't fail");
    let udp: Multiaddr = "/ip4/0.0.0.0/udp/0/quic-v1"
        .parse()
        .expect("Shouldn't fail");

    // let's automatically listen to the topics mention above.
    // all network interference must subscribe to these topics!
    controller.subscribe(JOB_TOPIC).await?;
    controller.subscribe(NODE_TOPIC).await?;

    // can we subscribe first before we listen?
    controller.start_listening(tcp).await;
    controller.start_listening(udp).await;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    dotenv().ok();

    // to collect user inputs for custom user preferences
    let cli = Cli::parse();

    // TODO: Ask Cli for the secret_key
    let secret_key = None;

    // If the user overrides a configuration path, then we'll use that, otherwise use default config directory location instead.
    let blend_config_path = cli
        .config_path
        .unwrap_or(BlenderConfig::get_default_config_path());
    let db_path = BlenderConfig::get_default_config_dir().join(constant::DATABASE_FILE_NAME);

    // initialize database connection
    let db: sqlx::Pool<sqlx::Sqlite> = config_sqlite_db(db_path)
        .await
        .expect("Must have database connection!");

    // must have working network services
    let (mut controller, receiver, server) = network::new(secret_key)
        .await
        .expect("Fail to start network service");

    // Network service is spun up on separate thread.
    spawn(async move {
        server.run().await;
    });

    if let Err(e) = setup_connection(&mut controller).await {
        eprintln!("Fail to setup connection! {e:?}");
    }

    let config = Some(blend_config_path); // expects a config path to load from.
    let manager = BlenderManager::load(config).expect("Must have blender configuration to load!");

    let server_settings = ServerSetting::load();
    let context = AppContext::new(manager, server_settings);

    // TODO: Restructure this to allow running client from GUI mode.
    let result = match cli.command {
        // run as client mode.
        Some(Commands::Client) => CliApp::new(context, &db).run(controller, receiver).await,
        // run as GUI mode.
        _ => {
            TauriApp::new(context.manager, &db)
                .clear_workers_collection()
                .await
                .run(controller, receiver)
                .await
        }
    };

    if let Err(e) = result {
        eprintln!("Received Network Error! {e:?}");
    }
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
