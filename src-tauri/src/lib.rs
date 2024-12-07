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
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use clap::Parser;
use models::app_state::AppState;
use models::network;
use services::{blend_farm::BlendFarm, cli_app::CliApp, display_app::DisplayApp};
use tokio::spawn;
// use tracing_subscriber::EnvFilter;

// TODO: Create a miro diagram structure of how this application suppose to work
// Need a mapping to explain how network should perform over intranet
// Need a mapping to explain how blender manager is used and invoked for the job

pub mod models;
pub mod routes;
pub mod services;

#[derive(Parser)]
struct Cli {
    #[arg(short, long, default_value = "false")]
    client: Option<bool>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    // let _ = tracing_subscriber::fmt()
    //     .with_env_filter(EnvFilter::from_default_env())
    //     .try_init();

    // to run custom behaviour
    let cli = Cli::parse();

    // must have working network services
    let (service, controller, receiver) =
        network::new().await.expect("Fail to start network service");

    // start network service async
    spawn(service.run());

    if let Err(e) = match cli.client {
        // run as client mode.
        Some(true) => CliApp::default().run(controller, receiver).await,
        // run as GUI mode.
        _ => DisplayApp::default().run(controller, receiver).await,
    } {
        eprintln!("Something went terribly wrong? {e:?}");
    }
}
