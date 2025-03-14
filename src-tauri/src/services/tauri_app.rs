use super::blend_farm::BlendFarm;
use crate::{
    domains::{job_store::JobStore, worker_store::WorkerStore},
    models::{
        app_state::AppState,
        computer_spec::ComputerSpec,
        job::{CreatedJobDto, JobEvent},
        message::{NetEvent, NetworkError},
        network::{NetworkController, HEARTBEAT, JOB, SPEC, STATUS},
        server_setting::ServerSetting,
        task::Task,
        worker::Worker,
    },
    routes::{job::*, remote_render::*, settings::*, util::*, worker::*},
};
use blender::{manager::Manager as BlenderManager,models::mode::Mode};
use libp2p::PeerId;
use maud::html;
use std::{collections::HashMap, ops::Range, sync::Arc, path::PathBuf, thread::sleep, time::Duration};
use tauri::{self, command, App, AppHandle, Emitter, Manager};
use tokio::{
    select, spawn, sync::{
        mpsc::{self, Receiver, Sender},
        Mutex, RwLock,
    }
};
use uuid::Uuid;

pub const WORKPLACE: &str = "workplace";

// This UI Command represent the top level UI that user clicks and interface with.
#[derive(Debug)]
pub enum UiCommand {
    StartJob(CreatedJobDto),
    StopJob(Uuid),
    UploadFile(PathBuf, String),
    RemoveJob(Uuid),
}

// TODO: make this user adjustable.
const MAX_BLOCK_SIZE: i32 = 30;

pub struct TauriApp {
    // I need the peer's address?
    peers: HashMap<PeerId, ComputerSpec>,
    worker_store: Arc<RwLock<(dyn WorkerStore + Send + Sync + 'static)>>,
    job_store: Arc<RwLock<(dyn JobStore + Send + Sync + 'static)>>,
}

#[command]
pub fn index() -> String {
    html! (
        div {
            div class="sidebar" {
                nav {
                    ul class="nav-menu-items" {
                        li key="manager" class="nav-bar" tauri-invoke="remote_render_page" hx-target=(format!("#{WORKPLACE}")) {
                            span { "Remote Render" }
                        };
                        li key="setting" class="nav-bar" tauri-invoke="setting_page" hx-target=(format!("#{WORKPLACE}")) {
                            span { "Setting" }
                        };
                    };
                };
                div {
                    h2 { "Computer Nodes" };
                    div class="group" id="workers" tauri-invoke="list_workers" hx-trigger="every 2s" hx-target="this" {};
                };
            };
            
            main tauri-invoke="remote_render_page" hx-trigger="load" hx-target="this" id=(WORKPLACE) {};
        }
    ).0
}

impl TauriApp {

    // Clear worker database before usage!
    pub async fn clear_workers_collection(self) -> Self {
        // A little closure hack
        {
            let mut db = self.worker_store.write().await;
            if let Err(e) = db.clear_worker().await{ 
                eprintln!("Error clearing worker database! {e:?}");
            } 
        }
        self
    }

    pub async fn new(
        worker_store: Arc<RwLock<(dyn WorkerStore + Send + Sync + 'static)>>,
        job_store: Arc<RwLock<(dyn JobStore + Send + Sync + 'static)>>,
    ) -> Self {
        Self {
            peers: Default::default(),
            worker_store,
            job_store,
        }
    }

    // Create a builder to make Tauri application
    fn config_tauri_builder(&self, to_network: Sender<UiCommand>) -> Result<App, tauri::Error> {
        // I would like to find a better way to update or append data to render_nodes,
        // "Do not communicate with shared memory"
        let builder = tauri::Builder::default()
            .plugin(tauri_plugin_cli::init())
            .plugin(tauri_plugin_os::init())
            .plugin(tauri_plugin_fs::init())
            .plugin(tauri_plugin_sql::Builder::default().build())
            .plugin(tauri_plugin_persisted_scope::init())
            .plugin(tauri_plugin_shell::init())
            .plugin(tauri_plugin_dialog::init())
            .setup(|_| Ok(()));

        let manager = Arc::new(RwLock::new(BlenderManager::load()));
        let setting = Arc::new(RwLock::new(ServerSetting::load()));

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
                index,
                open_path,
                select_directory,
                select_file,
                create_job,
                delete_job,
                get_job,
                setting_page,
                edit_settings,
                get_settings,
                update_settings,
                create_new_job,
                available_versions,
                remote_render_page,
                list_workers,
                list_jobs,
                get_worker,
                import_blend,
                update_output_field,
                add_blender_installation,
                list_blender_installed,
                remove_blender_installation,
                fetch_blender_installation,
            ])
            .build(tauri::generate_context!())
    }

    // because this is async, we can make our function wait for a new peers available.
    async fn get_idle_peers(&self) -> String {
        // this will destroy the vector anyway.
        // TODO: Impl. Round Robin or pick first idle worker, whichever have the most common hardware first in query?
        // This code doesn't quite make sense, at least not yet?
        loop {
            if let Some((.., spec)) = self.peers.clone().into_iter().nth(0) {
                return spec.host;
            }
            sleep(Duration::from_secs(1));
        }
    }

    fn generate_tasks(job: &CreatedJobDto, file_name: PathBuf, chunks: i32, hostname: &str) -> Vec<Task> {
        // mode may be removed soon, we'll see?
        let (time_start, time_end) = match &job.item.mode {
            Mode::Animation(anim) => (anim.start, anim.end),
            Mode::Frame(frame) => (frame.clone(), frame.clone()),
        };

        // What if it's in the negative? e.g. [-200, 2 ] ? would this result to -180 and what happen to the equation?
        let step = time_end - time_start;
        let max_step = step / chunks;
        let mut tasks = Vec::with_capacity(max_step as usize);

        // Problem: If i ask to render from 1 to 40, the end range is exclusive. Please make the range inclusive.
        for i in 0..=max_step {
            // current start block location.
            let block = time_start + i * chunks;

            let mut start = block;
            if i > 0 {
                // inclusive start
                start += 1;
            }

            let end = block + chunks;
            let end = match end.cmp(&time_end) {
                std::cmp::Ordering::Less => end,
                _ => time_end,
            };
            let range = Range { start, end };

            let task = Task::new(
                hostname.to_string(),
                job.id,
                file_name.clone(),
                job.item.get_version().clone(),
                range,
            );
            tasks.push(task);
        }

        tasks
    }

    // command received from UI
    async fn handle_command(&mut self, client: &mut NetworkController, cmd: UiCommand) {
        match cmd {
            // TODO: This may subject to change. 
            // Issue: What if the app restarts? We no longer provide the file after reboot.
            UiCommand::StartJob(job) => {
                // first make the file available on the network
                let file_name = job.item.project_file.file_name().unwrap();
                let path = job.item.project_file.clone();

                // Once job is initiated, we need to be able to provide the files for network distribution.
                client
                    .start_providing(file_name.to_str().unwrap().to_string(), path)
                    .await;
   
                let tasks = Self::generate_tasks(
                    &job,
                    PathBuf::from(file_name),
                    MAX_BLOCK_SIZE,
                    &client.hostname
                );

                // so here's the culprit. We're waiting for a peer to become idle and inactive waiting for the next job
                for task in tasks {
                    // problem here - I'm getting one client to do all of the rendering jobs, not the inactive one.
                    // Perform a round-robin selection instead.
                    let host = self.get_idle_peers().await; // this means I must wait for an active peers to become available?
                    println!("Sending task {:?} to {:?}", &task, &host);
                    let event = JobEvent::Render(task);
                    client.send_job_message(&host, event).await;
                }
            }
            UiCommand::UploadFile(path, file_name) => {
                client.start_providing(file_name, path).await;
            }
            UiCommand::StopJob(id) => {
                println!(
                    "Impl how to send a stop signal to stop the job and remove the job from queue {id:?}"
                );
            }
            UiCommand::RemoveJob(id) => {
                for (_, spec) in self.peers.clone() {
                    client.send_job_message(&spec.host, JobEvent::Remove(id)).await;
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
                // this may soon change.
                let handle = app_handle.read().await;
                handle
                    .emit("node_status", (peer_id.to_base58(), msg))
                    .unwrap();
            }
            NetEvent::NodeDiscovered(peer_id, spec) => {
                let worker = Worker::new(peer_id, spec.clone());
                let mut db = self.worker_store.write().await;
                if let Err(e) = db.add_worker(worker).await {
                    eprintln!("Error adding worker to database! {e:?}");
                }
                
                self.peers.insert(peer_id, spec);
                // let handle = app_handle.write().await;
                // emit a signal to query the data. 
                // TODO: See how this can be done: https://github.com/ChristianPavilonis/tauri-htmx-extension
                // let _ = handle.emit("worker_update");
            }
            NetEvent::NodeDisconnected(peer_id) => {
                let mut db = self.worker_store.write().await;
                // So the main issue is that there's no way to identify by the machine id?
                if let Err(e) = db.delete_worker(&peer_id).await {
                    eprintln!("Error deleting worker from database! {e:?}");
                }

                self.peers.remove(&peer_id);
            }
            NetEvent::InboundRequest { request, channel } => {
                if let Some(path) = client.providing_files.get(&request) {
                    client
                        .respond_file(std::fs::read(path).unwrap(), channel)
                        .await
                }
            }
            NetEvent::JobUpdate(_host, job_event) => match job_event {
                // when we receive a completed image, send a notification to the host and update job index to obtain the latest render image.
                JobEvent::ImageCompleted {
                    job_id,
                    frame: _,
                    file_name,
                } => {
                    // create a destination with respective job id path.
                    let destination = client.settings.render_dir.join(job_id.to_string());
                    if let Err(e) = async_std::fs::create_dir_all(destination.clone()).await {
                        println!("Issue creating temp job directory! {e:?}");
                    }
                    
                    // this is used to send update to the web app.
                    // let handle = app_handle.write().await;
                    // if let Err(e) = handle.emit(
                    //     "frame_update",
                    //     FrameUpdatePayload {
                    //         id,
                    //         frame,
                    //         file_name: file_name.clone(),
                    //     },
                    // ) {
                    //     eprintln!("Unable to send emit to app handler\n{e:?}");
                    // }

                    // Fetch the completed image file from the network
                    if let Ok(file) = client.get_file_from_peers(&file_name, &destination).await {
                        println!("File stored at {file:?}");
                        // let handle = app_handle.write().await;
                        // if let Err(e) = handle.emit("job_image_complete", (job_id, frame, file)) {
                        //     eprintln!("Fail to publish image completion emit to front end! {e:?}");
                        // }
                    }
                }

                // when a job is complete, check the poll for next available job queue?
                JobEvent::JobComplete => {} // Hmm how do I go about handling this one?

                // TODO: how do we handle error from node? What kind of errors are we expecting here and what can the host do about it?
                JobEvent::Error(job_error) => {
                    todo!("See how this can be replicated? {job_error:?}")
                }

                // send a render job
                // this will soon go away - host should not be receiving render jobs.
                JobEvent::Render(..) => {}
                // this will soon go away - host should not receive request job.
                JobEvent::RequestTask => {}
                // this will soon go away
                JobEvent::Remove(_) => {
                    // Should I do anything on the manager side? Shouldn't matter at this point?
                }
            },
            _ => {}, // println!("[TauriApp]: {:?}", event),
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
        client.subscribe_to_topic(client.hostname.clone()).await;

        // this channel is used to send command to the network, and receive network notification back.
        let (event, mut command) = mpsc::channel(32);

        // we send the sender to the tauri builder - which will send commands to "from_ui".
        let app = self
            .config_tauri_builder(event)
            .expect("Fail to build tauri app - Is there an active display session running?");

        // create a safe and mutable way to pass application handler to send notification from network event.
        // TODO: Get rid of this.
        let app_handle = Arc::new(RwLock::new(app.app_handle().clone()));

        // create a background loop to send and process network event
        spawn(async move {
            loop {
                select! {
                    Some(msg) = command.recv() => self.handle_command(&mut client, msg).await,
                    Some(event) = event_receiver.recv() => self.handle_net_event(&mut client, event, app_handle.clone()).await,
                }
            }
        });

        app.run(|_, _| {});
        Ok(())
    }
}
