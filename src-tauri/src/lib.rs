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

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
// it might be interesting and useful if there's a debug mode enabled?
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use crate::network::client::Client;
use crate::services::app_context::AppContext;
use blender_rs::manager::Manager as BlenderManager;
use blender_rs::models::blender_config::BlenderConfig;
use blender_rs::utils::{get_blend_config_default_location, get_config_folder_path};
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use iroh::endpoint::presets;
use iroh::{Endpoint, RelayMode, SecretKey};
use iroh_blobs::api::{TempTag, Store};
use iroh_blobs::format::collection::Collection;
use iroh_blobs::protocol::ALPN;
use iroh_blobs::provider::events::{ConnectMode, EventMask, EventSender};
use iroh_blobs::store::fs::FsStore;
use iroh_blobs::{BlobsProtocol, provider};
use rand::RngExt;
use services::{blend_farm::BlendFarm, server::Server, tauri_app::TauriApp};
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::error::Error;
use std::fs;
use std::path::Path;
use tokio::spawn;
use tracing_subscriber::EnvFilter;

// use figment::{
//     providers::{Env, Format, Json, Toml, Yaml},
//     Figment,
// };
// const SETTINGS_PATH_JSON: &str = "BlendFarm/BlenderManager.json";
// const SETTINGS_PATH_TOML: &str = "BlendFarm/BlenderManager.toml";
// const SETTINGS_PATH_YAML: &str = "BlendFarm/BlenderManager.yaml";

pub mod constant;
pub mod domains;
pub mod models;
pub mod network;
pub mod routes;
pub mod services;

#[derive(Debug, Parser)]
struct CommandLineArguments {
    #[command(subcommand)]
    command: Option<Commands>,
    #[arg(short, long, default_value=None)]
    secret_key: Option<u8>,
}

#[derive(Debug, Subcommand, Default)]
enum Commands {
    Service,
    #[default]
    Gui,
}

#[inline]
async fn config_sqlite_db(path: impl AsRef<Path>) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);
    SqlitePool::connect_with(options).await
}

/// Import from a file or directory into the database.
///
/// The returned tag always refers to a collection. If the input is a file, this
/// is a collection with a single blob, named like the file.
///
/// If the input is a directory, the collection contains all the files in the
/// directory.
async fn import(path: impl AsRef<Path>, db: &Store, jobs: Option<usize>) //-> (TempTag, u64, Collection)
{
    let parallelism = jobs.unwrap_or_else(1);
    let path = path.as_ref().canonicalize().expect("Path must be able to canonicalize. Data must be posioned!");

    let root = path.parent().expect("file path must exist within directory"); //.context("context get parent")?;

    let files = fs::read_dir(root).expect("Root must be a directory"); //WalkDir::new(path.clone()).into_iter();
    // ()
}

// design to setup network connection
async fn setup_connection(controller: &mut Client) -> Result<(), Box<dyn Error>> {
    let key = SecretKey::generate();
    let relay_mode = RelayMode::Default;
    let alpns_protocol = vec![ALPN.to_vec()];

    // TODO: Start providing list of completed rendered image files.
    // TODO: Start providing list of blender installed as bundle package.

    let builder = Endpoint::builder(presets::N0)
        .alpns(alpns_protocol)
        .secret_key(key)
        .relay_mode(relay_mode);

    // TODO: what is suffix?
    let suffix = rand::rng().random::<[u8; 16]>();

    // path to source file?
    let file_path = Path::new("./todo");

    // In the original code of sendme - this section of code is spawn inside async thread.
    // TODO: See if we need to refactor this piece? After we get this part working over network.
    let endpoint = builder.bind().await?;

    let store = FsStore::load(&file_path).await?;
    let blobs = BlobsProtocol::new(
        &store,
        Some(EventSender::new(
            progress_tx,
            EventMask {
                connected: ConnectMode::Notify,
                get: provider::events::RequestMode::NotifyLog, // TODO: Change this to Notify instead
                ..EventMask::DEFAULT
            },
        )),
    );

    let import_result = import()

    // if let Err(e) = controller.start_listening(tcp).await {
    //     eprintln!("Unable to listen using TCP provided address! {e:?}");
    // }

    // if let Err(e) = controller.start_listening(udp).await {
    //     eprintln!("Unable to listen using UDP provided address! {e:?}");
    // }

    // This will return some horrible results...
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    // TODO: figure out where/how to access tracing subscribers.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    // Loads local environment variable. (Used for database urls)
    dotenv().ok();

    // to collect user inputs for custom user preferences
    let cli = CommandLineArguments::parse();

    // TODO: consider using figment at this application scope level.
    //     let config = Figment::new()
    //         .merge(Toml::file(config_path.join(SETTINGS_PATH_TOML)))
    //         .merge(Yaml::file(config_path.join(SETTINGS_PATH_YAML)))
    //         .merge(Env::prefixed("BlendFarm_"))
    //         .join(Json::file(config_path.join(SETTINGS_PATH_JSON)))
    //         .extract::<BlenderConfig>()?;
    let config_path = get_blend_config_default_location().expect("Must have path to configs!");

    let config: BlenderConfig = match std::fs::read(config_path) {
        Ok(reader) => match serde_json::from_slice(&reader) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Unable to parse config file! Using default config instead. {e:?}");
                BlenderConfig::default()
            }
        },
        Err(e) => {
            eprintln!("Unable to open blender config file! {e:?}");
            BlenderConfig::default()
        }
    };

    // TODO: figure out how we can handle database path?
    // Expect to use yaml config loader.
    let db_path = get_config_folder_path()
        .expect("Must have path to configs!")
        .join(constant::DATABASE_FILE_NAME);

    // initialize database connection (We need a place to store persistent storage)
    let db = config_sqlite_db(db_path)
        .await
        .expect("Must have database connection!");

    // setup network services
    let (mut controller, receiver, server) = network::new(cli.secret_key)
        .await
        .expect("Fail to start network service");

    // Run Network service on separate thread.
    let network_thread = spawn(async move {
        server.run().await;
    });

    if let Err(e) = setup_connection(&mut controller).await {
        eprintln!("Fail to setup connection! {e:?}");
    }

    let manager = BlenderManager::load(config).expect("Must have blender configuration to load!");

    // This server settings is different than blender config.
    // Server Settings is used for Manager client only, to help organize and arrange file structure for completed render image results.
    let context = AppContext::new(manager);

    // TODO: Restructure this to allow running client from GUI mode.
    // TODO: Handle Receiver input here.
    let result = match cli.command {
        // run as client mode.
        Some(Commands::Service) => Server::new(context, &db).run(controller, receiver).await,
        // run as GUI mode.
        _ => {
            // could spawn in a separate thread?
            TauriApp::new(context.manager, &db)
                .clear_workers_collection()
                .await
                .run(controller, receiver)
                .await
        }
    };

    if let Err(e) = result {
        eprintln!("BlendFarm Error! {e:?}");
    }

    // abort network thread after closing.
    network_thread.abort();
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
