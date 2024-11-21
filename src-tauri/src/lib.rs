/*
Developer blog:
- Had a brain fart trying to figure out some ideas allowing me to run this application as either client or server
Originally thought of using Clap library to parse in input, but when I run `cargo tauri dev -- test` the application fail to compile due to unknown arguments when running web framework?
This issue has been solved by alllowing certain argument to run. By default it will try to launch the client user interface of the application.
Additionally, I need to check into the argument and see if there's a way we could just allow user to run --server without ui interface?
Interesting thoughts for sure
9/2/24 - Decided to rely on using Tauri plugin for cli commands and subcommands. Use that instead of clap. Since Tauri already incorporates Clap anyway.

- Had an idea that allows user remotely to locally add blender installation without using GUI interface,
This would serves two purposes - allow user to expressly select which blender version they can choose from the remote machine and
prevent multiple download instances for the node, in case the target machine does not have it pre-installed.

- Eventually, I will need to find a way to spin up a virtual machine and run blender farm on that machine to see about getting networking protocol working in place.
This will allow me to do two things - I can continue to develop without needing to fire up a remote machine to test this and
verify all packet works as intended while I can run the code in parallel to see if there's any issue I need to work overhead.
This might be another big project to work over the summer to understand how network works in Rust.

[F] - find a way to allow GUI interface to run as client mode for non cli users.
[F] - consider using channel to stream data https://v2.tauri.app/develop/calling-frontend/#channels
[F] - Before release - find a way to add updater  https://v2.tauri.app/plugin/updater/
*/
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::routes::job::{create_job, delete_job, list_jobs};
use crate::routes::remote_render::{delete_node, list_node, list_versions, ping_node};
use crate::routes::settings::{
    add_blender_installation, fetch_blender_installation, get_server_settings,
    list_blender_installation, remove_blender_installation, set_server_settings,
};
use blender::manager::Manager as BlenderManager;
use blender::models::home::BlenderHome;
use libp2p::core::Multiaddr;
use libp2p::futures::StreamExt;
use libp2p::gossipsub::{self, Message, MessageAuthenticity, MessageId, ValidationMode};
use libp2p::mdns;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::SwarmBuilder;
use models::app_state::AppState;
use models::server_setting::ServerSetting;
use services::network_service::NetworkService;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::{io, io::AsyncBufReadExt, select};
use tracing_subscriber::EnvFilter;

//TODO: Create a miro diagram structure of how this application suppose to work
// Need a mapping to explain how network should perform over intranet
// Need a mapping to explain how blender manager is used and invoked for the job

pub mod models;
pub mod routes;
pub mod services;

#[derive(NetworkBehaviour)]
struct MyBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

async fn create_swarm() -> Result<MyBehaviour, Box<dyn std::error::Error>> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    let duration = Duration::from_secs(60);

    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::tls::Config::new,
            libp2p::yamux::Config::default,
        )?
        .with_quic()
        .with_behaviour(|key| {
            let message_id_fn = |message: &Message| {
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                MessageId::from(s.finish().to_string())
            };

            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(10))
                .validation_mode(ValidationMode::Strict)
                .message_id_fn(message_id_fn)
                .build()
                .map_err(|msg| std::io::Error::new(std::io::ErrorKind::Other, msg))?;

            let gossipsub = gossipsub::Behaviour::new(
                MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )?;

            let mdns =
                mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())?;
            Ok(MyBehaviour { gossipsub, mdns })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(duration))
        .build();

    let topic = gossipsub::IdentTopic::new("test-net");
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    let mut stdin = io::BufReader::new(io::stdin()).lines();

    let tcp: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse()?;
    let udp: Multiaddr = "/ip4/0.0.0.0/udp/0/quic-v1".parse()?;
    swarm.listen_on(tcp)?;
    swarm.listen_on(udp)?;

    loop {
        select! {
            Ok(Some(line)) = stdin.next_line() => {
                if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic.clone(), line.as_bytes()) {
                    println!("Publish error: {e:?}");
                }
            },
            event = swarm.select_next_some() => match event {
                SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, _multiaddr) in list {
                        println!("mDNS discovered a new peer: {peer_id}");
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                    for( peer_id, _multiaddr) in list {
                        println!("mDNS discover peer has expired: {peer_id}");
                        swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source: peer_id,
                    message_id: id,
                    message,
                })) => println!("Got message '{}' with id: {id} from peer: {peer_id}", String::from_utf8_lossy(&message.data))
                ,
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Local node is listening on {address}");
                },
                _ => {}
            }
        }
    }
}

// when the app starts up, I would need to have access to configs. Config is loaded from json file - which can be access by user or program - it must be validate first before anything,
fn client() {
    // I would like to find a better way to update or append data to render_nodes,
    // "Do not communicate with shared memory"
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_cli::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_persisted_scope::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|_| Ok(()));

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    // I'm having problem trying to separate this call from client.
    // I want to be able to run either server _or_ client via a cli switch.
    // Would like to know how I can get around this?

    // somehow I need a closure for the spawn to take place, it expects FnOnce, but I need this function to be awaitable.
    //] todo - find a way to call async function within sync thread? I want to be able to invoke async call to start the network service on a separate thread. However, limitation of rust prevents me from running async method in sync function.

    runtime.spawn(async move {
        let _ = create_swarm().await;
    });

    // todo is there a better way to handle blender objects?
    let manager = Arc::new(RwLock::new(BlenderManager::load()));
    let source = Arc::new(RwLock::new(
        BlenderHome::new()
            .expect("Unable to connect to blender.org, are you connect to the internet?"),
    ));
    let setting = Arc::new(RwLock::new(ServerSetting::load()));

    // for network service, consider making a box pointer instead. this Arc<RwLock<T>> is driving me nuts with Tauri.
    // Do consider adding blender manager and blender home in app state instead.
    let app_state = AppState {
        manager,
        blender_source: source,
        setting,
        jobs: Vec::new(),
    };

    let app = builder
        .manage(Mutex::new(app_state))
        .invoke_handler(tauri::generate_handler![
            create_job,
            delete_node,
            delete_job,
            list_node,
            list_jobs,
            list_versions,
            get_server_settings,
            set_server_settings,
            ping_node,
            add_blender_installation,
            list_blender_installation,
            remove_blender_installation,
            fetch_blender_installation,
        ])
        .build(tauri::generate_context!())
        .expect("Unable to build tauri app!");

    // match app.cli().matches() {
    //     // `matches` here is a Struct with { args, subcommand }.
    //     // `args` is `HashMap<String, ArgData>` where `ArgData` is a struct with { value, occurrences }.
    //     // `subcommand` is `Option<Box<SubcommandMatches>>` where `SubcommandMatches` is a struct with { name, matches }.
    //     // cargo tauri dev -- -- -c
    //     Ok(matches) => {
    //         dbg!(&matches);
    //         if matches.args.get("client").unwrap().occurrences >= 1 {
    //             // run client mode instead.
    //             spawn(run_client());
    //         }
    //     }
    //     Err(e) => {
    //         dbg!(e);
    //     }
    // };

    app.run(|_, _| {});
    // this never gets called?
    println!("After run");
}

pub async fn start_network_service() -> Result<(), Box<dyn std::error::Error>> {
    NetworkService::new(100).await
}

// not sure why I'm getting a lint warning about the mobile macro? Need to bug the dev and see if this macro has changed.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // TODO: Find a way to make use of Tauri cli commands to run as client.
    // TODO: It would be nice to include command line utility to let the user add blender installation from remotely.
    // The command line would take an argument of --add or -a to append local blender installation from the local machine to the configurations.
    client();
    // task.await.unwrap();
}
