use crate::{
    domains::{job_store::JobStore, worker_store::WorkerStore},
    models::{
        app_state::AppState, computer_spec::ComputerSpec, job::{Job, JobEvent}, message::{NetEvent, NetworkError}, network::{NetworkController, HEARTBEAT, JOB, SPEC, STATUS}, server_setting::ServerSetting, task::Task, worker::Worker
    },
    routes::{job::*, remote_render::*, settings::*},
};
use blender::manager::Manager as BlenderManager;
use libp2p::PeerId;
use surrealdb::{engine::local::Db, Surreal};
use std::{collections::{HashMap, VecDeque}, path::PathBuf, sync::Arc};
use tauri::{self, App, AppHandle, Emitter, Manager};
use tokio::{
    select, spawn,
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex, RwLock,
    },
};
use uuid::Uuid;
use blender::models::mode::Mode;

/*
    Dev blog:
        Consider looking into real_time_sqlx to create a realtime database update to frontend for any message queue/updates from sqlx.
        Once I get sqlx implemented.
        https://docs.rs/real-time-sqlx/latest/real_time_sqlx/
        https://www.reddit.com/r/rust/comments/1gvslni/realtimesqlx_a_sqlxsqlitebased_realtime_query/
*/

// This UI Command represent the top level UI that user clicks and interface with.
#[derive(Debug)]
pub enum UiCommand {
    StartJob(Job),
    StopJob(Uuid),
    UploadFile(PathBuf, String),
    RemoveJob(Uuid),
}

use super::{blend_farm::BlendFarm, data_store::{surrealdb_job_store::SurrealDbJobStore, surrealdb_worker_store::SurrealDbWorkerStore}};

// TODO: make this user adjustable.
const MAX_BLOCK_SIZE: i32 = 30;

pub struct TauriApp {
    peers: HashMap<PeerId, ComputerSpec>,
    worker_store: Arc<RwLock<(dyn WorkerStore + Send + Sync + 'static)>>,
    job_store: Arc<RwLock<(dyn JobStore + Send + Sync + 'static)>>,
}

impl TauriApp {
        pub fn new(
            db: Arc<RwLock<Surreal<Db>>>
    ) -> Self {

        let job_store = Arc::new(RwLock::new(SurrealDbJobStore::new(db.clone())));
        let worker_store = Arc::new(RwLock::new(SurrealDbWorkerStore::new(db)));

        Self {
            peers: Default::default(),
            worker_store,
            job_store,
        }
    }

    // Create a builder to make Tauri application
    fn config_tauri_builder(&self, to_network: Sender<UiCommand>) -> Result<App, tauri::Error> {
        let server_settings = ServerSetting::load();

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

        let manager = Arc::new(RwLock::new(BlenderManager::load()));
        let setting = Arc::new(RwLock::new(server_settings));

        // here we're setting the sender command to app state before the builder.
        let app_state = AppState {
            manager,
            to_network,
            setting,
            job_db: self.job_store.clone(),
            worker_db: self.worker_store.clone(),
        };

        let mut_app_state = Mutex::new(app_state);

        builder
            .manage(mut_app_state)
            .invoke_handler(tauri::generate_handler![
                create_job,
                delete_job,
                list_jobs,
                list_versions,
                import_blend,
                get_server_settings,
                set_server_settings,
                add_blender_installation,
                list_blender_installation,
                remove_blender_installation,
                fetch_blender_installation,
            ])
            .build(tauri::generate_context!())
    }

    async fn get_idle_peers(&self) -> PeerId {   
        // this will destroy the vector anyway.
        // TODO: Impl. Round Robin or pick first idle worker, whichever have the most common hardware first in query?
        match self.peers.clone().into_iter().nth(0) {
            Some((peer, ..)) => peer,
            None => PeerId::random(),
        }
    }



    // command received from UI
    async fn handle_ui_command(&mut self, client: &mut NetworkController, cmd: UiCommand) {
        match cmd {
            UiCommand::StartJob(job) => {
                // first make the file available on the network
                let file_name = job.get_file_name().to_owned();
                let path = job.get_project_path().clone();
                
                // Make the file providable.
                client.start_providing(file_name.clone(), path).await;
                
                
                match job.mode {
                    Mode::Frame(frame) => {
                        // send one peer the job.
                        let peer = self.get_idle_peers().await; 
                        let requestor = client.get_local_peer().await; 
                        // Create one task 
                        let chunks = [frame].into();
                        let task = Task::new(requestor, job.id, file_name.clone(), job.get_version().clone(), chunks);
                        let event = JobEvent::Render(task);
                        client.send_job_message(peer, event).await;
                    },
                    Mode::Animation(ref range) => {
                        // What if it's in the negative? e.g. [-200, 2 ] ? would this result to -180 and what happen to the equation?
                        let offset = range.end - range.start;
                        let requestor = client.get_local_peer().await;

                        // TODO: Check for negative? What will that impact    
                        // the rule should be ( ( frame - offset ) % SIZE + 1 ) * SIZE + offset 
                        for i in 0..= offset % MAX_BLOCK_SIZE {
                            let block = i * MAX_BLOCK_SIZE;
                            let start = block + offset;
                            let end = start + MAX_BLOCK_SIZE;
                            let end = match end.cmp(&range.end) {
                                std::cmp::Ordering::Less => end,
                                _ => range.end,
                            };

                            let frames: VecDeque<i32> = (start..=end).map(i32::from).collect();   // E.g. (101..=236)
                            let task = Task::new(requestor, job.id, file_name.clone(), job.get_version().clone(), frames);
                            let peer = self.get_idle_peers().await; // this means I must wait for a active peers?
                            let event = JobEvent::Render(task);
                            client.send_job_message(peer, event).await;
                        }
                    }
                }
            }
            UiCommand::UploadFile(path, file_name) => {
                client.start_providing(file_name, path).await;
            }
            UiCommand::StopJob(id) => {
                todo!(
                    "Impl how to send a stop signal to stop the job and remove the job from queue {id:?}"
                );
            }
            UiCommand::RemoveJob(id) => {
                for (peer, _) in self.peers.clone() {
                    client.send_job_message(peer, JobEvent::Remove(id)).await;
                }
            }
        }
    }

    // commands received from network
    async fn handle_net_event(
        &mut self,
    client: &mut NetworkController,
        event: NetEvent,
        // This is currently used to receive worker's status update. We do not want to store this information in the database, instead it should be sent only when the application is available.
        app_handle: Arc<RwLock<AppHandle>>,
    ) {
        match event {
            NetEvent::Status(peer_id, msg) => {
                let handle = app_handle.read().await;
                handle
                    .emit("node_status", (peer_id.to_base58(), msg))
                    .unwrap();
            }
            NetEvent::NodeDiscovered(peer_id, spec) => {
                // let handle = app_handle.read().await;
                // handle
                //     .emit("node_discover", (peer_id.to_base58(), comp_spec.clone()))
                //     .unwrap();
                let worker = Worker::new(peer_id.to_base58(), spec.clone());
                let mut db = self.worker_store.write().await;
                if let Err(e) = db.add_worker(worker).await {
                    eprintln!("Error adding worker to database! {e:?}");
                }
                self.peers.insert(peer_id, spec);
            }
            NetEvent::NodeDisconnected(_) => {}
            NetEvent::InboundRequest { request, channel } => {
                if let Some(path) = client.providing_files.get(&request) {
                    println!("Sending client file {path:?}");
                    client
                        .respond_file(std::fs::read(path).unwrap(), channel)
                        .await
                }
            }
            NetEvent::JobUpdate(.., job_event) => match job_event {
                // when we receive a completed image, send a notification to the host and update job index to obtain the latest render image.
                JobEvent::ImageCompleted {
                    job_id: id,
                    frame,
                    file_name,
                } => {
                    // create a destination with respective job id path.
                    let destination = client.settings.render_dir.join(id.to_string());
                    if let Err(e) = async_std::fs::create_dir_all(destination.clone()).await {
                        println!("Issue creating temp job directory! {e:?}");
                    }

                    // Fetch the completed image file from the network
                    if let Ok(file) = client.get_file_from_peers(&file_name, &destination).await {
                        let handle = app_handle.write().await;
                        if let Err(e) = handle.emit("job_image_complete", (id, frame, file)) {
                            eprintln!("Fail to publish image completion emit to front end! {e:?}");
                        }
                    }
                }

                // when a job is complete, check the poll for next available job queue?
                JobEvent::JobComplete => {} // Hmm how do I go about handling this one?
                // TODO: how do we handle error from node? What kind of errors are we expecting here and what can the host do about it?
                JobEvent::Error(job_error) => {
                    todo!("See how this can be replicated? {job_error:?}")
                }
                // send a render job
                JobEvent::Render(_) => {} // should be ignored.
                // Received a request job?
                JobEvent::RequestJob => {}
                // Remove Job entry from database
                JobEvent::Remove(_) => {
                    // Should I do anything on the manager side? Shouldn't matter at this point?
                }
            },
            _ => println!("{:?}", event),
        }
    }
}

#[async_trait::async_trait]
impl BlendFarm for TauriApp {
    async fn run(
        mut self,
        mut client: NetworkController,
        mut event_receiver: Receiver<NetEvent>,
    ) -> Result<(), NetworkError> {
        // for application side, we will subscribe to message event that's important to us to intercept.
        client.subscribe_to_topic(SPEC.to_owned()).await;
        client.subscribe_to_topic(HEARTBEAT.to_owned()).await; 
        client.subscribe_to_topic(STATUS.to_owned()).await;
        client.subscribe_to_topic(JOB.to_owned()).await; // This might get changed? we'll see.

        // this channel is used to send command to the network, and receive network notification back.
        let (to_network, mut from_ui) = mpsc::channel(32);

        // we send the sender to the tauri builder - which will send commands to "from_ui".
        let app = self
            .config_tauri_builder(to_network)
            .expect("Fail to build tauri app - Is there an active display session running?");

        // create a safe and mutable way to pass application handler to send notification from network event.
        // TODO: Get rid of this.
        let app_handle = Arc::new(RwLock::new(app.app_handle().clone()));

        // create a background loop to send and process network event
        spawn(async move {
            loop {
                select! {
                    Some(msg) = from_ui.recv() => self.handle_ui_command(&mut client, msg).await,
                    Some(event) = event_receiver.recv() => self.handle_net_event(&mut client, event, app_handle.clone()).await,
                }
            }
        });

        app.run(|_, _| {});

        Ok(())
    }
}
