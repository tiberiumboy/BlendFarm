/* DEV Blog

    Issue: files provider are stored in memory, and do not recover after application restart. 
        - mitigate this by using a persistent storage solution instead of memory storage.

    Issue: Cannot debug this application unless it is built completely. See if there's a way to run debug mode without building the app entirely.
*/

use super::{blend_farm::BlendFarm, data_store::{sqlite_job_store::SqliteJobStore, sqlite_worker_store::SqliteWorkerStore}};
use crate::{
    domains::{job_store::JobStore, worker_store::WorkerStore},
    models::{
        app_state::AppState, 
        computer_spec::ComputerSpec, 
        job::{
            CreatedJobDto, 
            JobEvent, 
            JobId, 
            NewJobDto
        }, 
        message::{Event, NetworkError}, 
        network::{NetworkController, NodeEvent, ProviderRule}, 
        server_setting::ServerSetting, 
        task::Task, 
        worker::Worker
    },
    routes::{job::*, remote_render::*, settings::*, util::*, worker::*},
};
use futures::{channel::mpsc::{self, Sender}, SinkExt, StreamExt};
use blender::{blender::Blender, manager::Manager as BlenderManager, models::mode::RenderMode};
use libp2p::PeerId;
use maud::html;
use semver::Version;
use sqlx::{Pool, Sqlite};
use std::{collections::HashMap, ops::Range, path::PathBuf, str::FromStr};
use tauri::{self, command};
use tokio::{select, spawn, sync::Mutex};

pub const WORKPLACE: &str = "workplace";

#[derive(Debug)]
pub enum SettingsAction {
    Get(Sender<ServerSetting>),
    Update(ServerSetting),
}

impl PartialEq for SettingsAction {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Get(..), Self::Get(..)) => true,
            (Self::Update(l0), Self::Update(r0)) => l0 == r0,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub enum BlenderAction {
    Add(PathBuf),
    List(Sender<Option<Vec<Blender>>>),
    Get(Version, Sender<Option<Blender>>),
    Disconnect(Blender),    // detach links associated with file path, but does not delete local installation!
    Remove(Blender),    // deletes local installation of blender, use it as last resort option. (E.g. force cache clear/reinstall/ corrupted copy)
}

impl PartialEq for BlenderAction {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Add(l0), Self::Add(r0)) => l0 == r0,
            (Self::List(..), Self::List(..)) => true,
            (Self::Get(l0, ..), Self::Get(r0, ..)) => l0 == r0,
            (Self::Remove(l0), Self::Remove(r0)) => l0 == r0,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub enum JobAction {
    Start(JobId),
    Stop(JobId),
    Get(JobId, Sender<Option<CreatedJobDto>>),
    Remove(JobId),
    List(Sender<Option<Vec<CreatedJobDto>>>),
    Advertise(NewJobDto)
}

impl PartialEq for JobAction {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Start(l0), Self::Start(r0)) => l0 == r0,
            (Self::Stop(l0), Self::Stop(r0)) => l0 == r0,
            (Self::Get(l0, ..), Self::Get(r0, ..)) => l0 == r0,
            (Self::Remove(l0), Self::Remove(r0)) => l0 == r0,
            (Self::List(..), Self::List(..)) => true,
            (Self::Advertise(l0), Self::Advertise(r0)) => l0 == r0,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub enum WorkerAction {
    Get(PeerId, Sender<Option<Worker>>),
    List(Sender<Option<Vec<Worker>>>),
}

impl PartialEq for WorkerAction {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Get(l0, ..), Self::Get(r0, ..)) => l0 == r0,
            (Self::List(..), Self::List(..)) => true,
            _ => false,
        }
    }
}
    
    #[derive(Debug, PartialEq)]
    pub enum UiCommand {
        Job(JobAction),
        UploadFile(PathBuf),
        Worker(WorkerAction),
        Settings(SettingsAction),
    Blender(BlenderAction),
}

// custom implementation was required to omit Sender being viewed as foreign item type. (Sender from futures-channel does not impl PartialEq)
// in this case of PartialEq, We do not care about comparing Sender, so Sender only variant returns true by default. (enum matches enum we're looking for)
// impl PartialEq for UiCommand {
//     fn eq(&self, other: &Self) -> bool {
//         match (self, other) {
//             (Self::AddJobToNetwork(l0), Self::AddJobToNetwork(r0)) => l0 == r0,
//             (Self::StartJob(l0), Self::StartJob(r0)) => l0 == r0,
//             (Self::StopJob(l0), Self::StopJob(r0)) => l0 == r0,
//             (Self::GetJob(l0, ..), Self::GetJob(r0, ..)) => l0.eq(r0),
//             (Self::UploadFile(l0), Self::UploadFile(r0)) => l0 == r0,
//             (Self::RemoveJob(l0), Self::RemoveJob(r0)) => l0 == r0,
//             (Self::ListJobs(..), Self::ListJobs(..)) => true,
//             (Self::ListWorker(..), Self::ListWorker(..)) => true,
//             (Self::GetWorker(l0, ..), Self::GetWorker(r0, ..)) => l0 == r0,
//             _ => false,
//         }
//     }
// }

pub struct TauriApp{
    // I need the peer's address?
    peers: HashMap<PeerId, ComputerSpec>,
    worker_store: SqliteWorkerStore,
    job_store: SqliteJobStore,
    settings: ServerSetting,
    manager: BlenderManager
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
                    // hx-trigger="every 10s" - omitting this as this was spamming console log
                    div class="group" id="workers" tauri-invoke="list_workers" hx-target="this" {};
                };
            };
            
            main tauri-invoke="remote_render_page" hx-trigger="load" hx-target="this" id=(WORKPLACE) {};
        }
    ).0
}

impl TauriApp {

    // Clear worker database before usage!
    pub async fn clear_workers_collection(mut self) -> Self {
        if let Err(e) = self.worker_store.clear_worker().await{ 
            eprintln!("Error clearing worker database! {e:?}");
        } 
        self
    }

    pub async fn new(
        pool: &Pool<Sqlite>,
    ) -> Self {

        Self {
            peers: Default::default(),
            worker_store: SqliteWorkerStore::new(pool.clone()),
            job_store: SqliteJobStore::new(pool.clone()),
            settings: ServerSetting::load(),
            manager: BlenderManager::load()
        }
    }

    // Create a builder to make Tauri application
    // Let's just use the controller in here anyway.
    pub async fn config_tauri_builder<R: tauri::Runtime>(&self, builder: tauri::Builder<R>, invoke: Sender<UiCommand>) -> Result<tauri::App<R>, tauri::Error> {
        // I would like to find a better way to update or append data to render_nodes,
        // "Do not communicate with shared memory"
        let app_state = AppState { invoke };
        let mut_app_state = Mutex::new(app_state);
        Ok(builder
            .plugin(tauri_plugin_cli::init())
            .plugin(tauri_plugin_os::init())
            .plugin(tauri_plugin_fs::init())
            .plugin(tauri_plugin_persisted_scope::init())
            .plugin(tauri_plugin_shell::init())
            .plugin(tauri_plugin_dialog::init())
            .setup(|_| Ok(()))
            .manage(mut_app_state)
            .invoke_handler(tauri::generate_handler![
                index,
                open_path,
                open_dir,
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
                delete_blender,
                fetch_blender_installation,
            ])
            .build(tauri::generate_context!("tauri.conf.json"))?)
    }

    // because this is async, we can make our function wait for a new peers available.
    /* 
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
    */

    // The idea here is to generate new task based on job creation.
    // TODO: Explain the expect behaviour for this method before reference it.
    #[allow(dead_code)]
    fn generate_tasks(job: &CreatedJobDto, file_name: PathBuf, chunks: i32) -> Vec<Task> {
        // mode may be removed soon, we'll see?
        let (time_start, time_end) = match &job.item.mode {
            RenderMode::Animation(anim) => (anim.start, anim.end),
            RenderMode::Frame(frame) => (frame.clone(), frame.clone()),
        };

        // What if it's in the negative? e.g. [-200, 2 ] ? would this result to -180 and what happen to the equation?
        let step = time_end - time_start;
        let max_step = step / chunks;
        let mut tasks = Vec::with_capacity(max_step as usize);

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
                job.id,
                file_name.clone(),
                job.item.get_version().clone(),
                range
            );
            tasks.push(task);
        }

        tasks
    }

    async fn handle_job_command(&mut self, job_action: JobAction, client: &mut NetworkController ) {
            match job_action {
            JobAction::Start(job_id) => {
                // first see if we have the job in the database?
                let job = match self.job_store.get_job(&job_id).await {
                    Ok(job) => job,
                    Err(e) => {
                        eprintln!("No Job record found! Skipping! {e:?}");
                        return ();
                    }
                };

                // first make the file available on the network
                let _file_name = job.item.project_file.file_name().unwrap();// this is &OsStr
                let path = job.item.project_file.clone();

                // Once job is initiated, we need to be able to provide the files for network distribution.
                let _provider = ProviderRule::Default(path);

                // where does the client come from?
                // TODO: Figure out where the client is associated with and how can we access it from here?
                /* 
                client.start_providing(&provider).await;

                let tasks = Self::generate_tasks(
                    &job,
                    PathBuf::from(file_name),
                    MAX_FRAME_CHUNK_SIZE,
                    &client.hostname
                );

                // so here's the culprit. We're waiting for a peer to become idle and inactive waiting for the next job
                // TODO how is this still pending?
                for task in tasks {
                    // problem here - I'm getting one client to do all of the rendering jobs, not the inactive one.
                    // Perform a round-robin selection instead.
                    let host = self.get_idle_peers().await; // this means I must wait for an active peers to become available?
                    println!("Sending task to {:?} \nJob Id: {:?} \nRange( {} - {} )\n", &host, &task.job_id, &task.range.start, &task.range.end);
                    client.send_job_event(Some(host.clone()), JobEvent::Render(task)).await;
                }
                */
            },
            JobAction::Stop(id) => {
                let signal = JobEvent::Remove(id);
                client.send_job_event(signal).await;
            },
            JobAction::Get(job_id, mut sender) => {
                let result = self.job_store.get_job(&job_id).await;
                if let Err(e) = &result {
                    eprintln!("Job store reported an error: {e:?}");
                }
                if let Err(e) = sender.send(result.ok()).await {
                    eprintln!("Unable to get a job!: {e:?}");
                }
            },
            JobAction::Remove(job_id) => {
                if let Err(e) = self.job_store.delete_job(&job_id).await {
                    eprintln!("Receiver/sender should not be dropped! {e:?}");
                }
                client.send_job_event(JobEvent::Remove(job_id)).await;   
            },
            JobAction::List(mut sender) => {
                /*  
                    There's something wrong with this datastructure. 
                    On first call, this command works as expected,
                    however additional call afterward does not let this function continue or invoke?
                    I must be waiting for something here?
                */
                let result = match self.job_store.list_all().await {
                    Ok(jobs) => {
                        if jobs.is_empty() {
                            None
                        } else {
                            Some(jobs)
                        }
                    },
                    Err(e) => {
                        eprintln!("Unable to send list of jobs: {e:?}");
                        None
                    }
                };
                            
                if let Err(e) = sender.send(result).await {
                    eprintln!("Fail to send data back! {e:?}");
                }
            },
            JobAction::Advertise(job) => // Here we will simply add the job to the database, and let client poll them!
                if let Err(e) = self.job_store.add_job(job).await {
                    eprintln!("Unable to add job! Encounter database error: {e:}");
                }
        }
    }

    async fn handle_blender_command(&mut self, blender_action: BlenderAction ) {
        match blender_action {
            BlenderAction::Add(_blender) => {
                todo!("impl adding blender?");
            },
            BlenderAction::List(mut sender) => {
                let localblenders = self.manager.get_blenders().to_owned();
                if let Err(e) = sender.send(Some(localblenders)).await {
                    eprintln!("Fail to send back list of blenders to caller! {e:?}");
                }

                // TODO: What's the difference?
                /* 
                let mut versions = Vec::new();

                // fetch local installation first.
                let mut local = self.manager
                    .get_blenders()
                    .iter()
                    .map(|b| b.get_version().clone())
                    .collect::<Vec<Version>>();

                if !local.is_empty() {
                    versions.append(&mut local);
                }

                // then display the rest of the download list
                if let Some(downloads) = self.manager.fetch_download_list() {
                    let mut item = downloads
                        .iter()
                        .map(|d| d.get_version().clone())
                        .collect::<Vec<Version>>();
                    versions.append(&mut item);
                };

                sender.send(Some(versions)).await;

                */
            },
            BlenderAction::Get(version, mut sender) => {
                let result = self.manager.fetch_blender(&version);
                match result {
                    Ok(blender) => { 
                        if let Err(e) = sender.send(Some(blender)).await {
                            eprintln!("Fail to send result back to caller! {e:?}");
                        } },
                    Err(e) => {
                        eprintln!("Fail to fetch blender! {e:?}");
                        if let Err(e) = sender.send(None).await {
                            eprintln!("Fail to send result back to caller! {e:?}");
                        }
                    }
                };
            },
            // I'm not really sure what this one is suppose to be?
            BlenderAction::Disconnect(..) => todo!(),
            // neither this one...
            BlenderAction::Remove(..) => todo!(),
        }
    }

    async fn handle_worker_command(&mut self, worker_action: WorkerAction) {
        match worker_action {
            WorkerAction::Get(peer_id,mut sender) => {
                let result = sender.send(self.worker_store.get_worker(&peer_id).await).await;
                if let Err(e) = result {
                    eprintln!("Unable to get worker!: {e:?}");
                }
            },
            WorkerAction::List(mut sender) => {
                let result = sender.send(self.worker_store.list_worker().await.ok()).await;
                if let Err(e) = result {
                    eprintln!("Unable to send list of workers: {e:?}");
                }
            },
        }
    }

    async fn handle_setting_command(&mut self, setting_action: SettingsAction) {
        match setting_action {
            SettingsAction::Get(mut sender) => {
                if let Err(e) = sender.send(self.settings.clone()).await {
                    eprintln!("Fail to send to invoker! {e:?}");
                }
            }
            SettingsAction::Update(new_settings) => {
                self.settings = new_settings;
                self.settings.save();
            }
        }
    }

    // command received from UI
    async fn handle_command(&mut self, client: &mut NetworkController, cmd: UiCommand) {
        // println!("Received command from UI: {cmd:?}");
        match cmd {
            // could this be used as a trait?
            UiCommand::Blender(blender_action) => self.handle_blender_command(blender_action).await,
            UiCommand::Settings(setting_action) => self.handle_setting_command(setting_action).await,
            UiCommand::Job(job_action) => self.handle_job_command(job_action, client).await,
            UiCommand::Worker(worker_action) => self.handle_worker_command(worker_action).await,
            UiCommand::UploadFile(path) => {
                // this is design to notify the network controller to start advertise provided file path
                let provider = ProviderRule::Default(path);
                client.start_providing(&provider).await;
            }
        }
    }

    // commands received from network
    async fn handle_net_event(
        &mut self,
        client: &mut NetworkController,
        event: Event,
    ) {
        match event {
            Event::NodeStatus(node_status) => match node_status {
                NodeEvent::Hello(peer_id_string, spec) => {
                    let peer_id = PeerId::from_str(&peer_id_string).expect("Peer id should be valid");
                    let worker = Worker::new(peer_id.clone(), spec.clone());
                    // append new worker to database store
                    if let Err(e) = self.worker_store.add_worker(worker).await {
                        eprintln!("Error adding worker to database! {e:?}");
                    }
                    
                    self.peers.insert(peer_id, spec);
                    // let handle = app_handle.write().await;
                    // emit a signal to query the data. 
                    // TODO: See how this can be done: https://github.com/ChristianPavilonis/tauri-htmx-extension
                    // let _ = handle.emit("worker_update");
                },
                // concerning - this String could be anything?
                // TODO: Find a better way to get around this.
                NodeEvent::Disconnected{ peer_id, reason } => {
                    if let Some(msg) = reason {
                        eprintln!("Node disconnected with reason!\n {msg}");
                    }
                    
                    // So the main issue is that there's no way to identify by the machine id?
                    let peer_id = PeerId::from_str(&peer_id).expect("Received invalid peer_id string!");
                    
                    // probably best to mark the node "inactive" instead?
                    if let Err(e) = self.worker_store.delete_worker(&peer_id).await {
                        eprintln!("Error deleting worker from database! {e:?}");
                    }
                    
                    self.peers.remove(&peer_id);
                },
                // this is the same as saying down in the garbage disposal. Anything goes here. Do not trust data source here!
                NodeEvent::BlenderStatus(blend_event) => println!("Blender Status Received: {blend_event:?}"),
            },
            
            // let me figure out what's going on here.
            // a network sent us a inbound request - reply back with the file data in channel.
            // yeah I wonder why we can't move this inside network class?
            Event::InboundRequest { request, channel } => {    
                self.handle_inbound_request(client, request, channel).await;
            }

            Event::JobUpdate(job_event) => match job_event {
                // when we receive a completed image, send a notification to the host and update job index to obtain the latest render image.
                JobEvent::ImageCompleted {
                    job_id,
                    frame: _,
                    file_name,
                } => {
                    // create a destination with respective job id path.
                    let destination = self.settings.render_dir.join(job_id.to_string());
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
                // when a task is complete, check the poll for next available job queue?
                JobEvent::TaskComplete => {
                    println!("Received Task Completed! Do something about this!");
                }

                // TODO: how do we handle error from node? What kind of errors are we expecting here and what can the host do about it?
                JobEvent::Error(job_error) => {
                    todo!("See how this can be replicated? {job_error:?}")
                }

                // send a render job
                // this will soon go away - host should not be receiving render jobs.
                JobEvent::Render(..) => {}
                // this will soon go away - host should not receive request job.
                JobEvent::RequestTask => {
                    // Node have exhaust all of queue. Check and see if we can create or distribute pending jobs.
                    todo!("A node from the network request more task to work on. More likely it was recently created or added after job was initially created.");
                }
                // this will soon go away
                JobEvent::Failed(msg) => {
                    eprintln!("Job failed! {msg}");
                }
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
        mut event_receiver: futures::channel::mpsc::Receiver<Event>,
    ) -> Result<(), NetworkError> {

        // this channel is used to send command to the network, and receive network notification back.
        // ok where is this used?
        let (event, mut command) = mpsc::channel(32);
        
        // we send the sender to the tauri builder - which will send commands to "from_ui".
        let app = self
            .config_tauri_builder(tauri::Builder::default(), event)
            .await
            .expect("Fail to build tauri app - Is there an active display session running?");

        // background thread to handle network process
        spawn(async move {
            loop {
                select! {
                    msg = command.select_next_some() => self.handle_command(&mut client, msg).await,
                    event = event_receiver.select_next_some() => self.handle_net_event(&mut client, event).await,
                }
            }
        });

        app.run(|_, _| {});
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::config_sqlite_db;
    use super::*;
    
    async fn get_sqlite_conn() -> Pool<Sqlite> {
        let pool = config_sqlite_db().await;
        assert!(pool.is_ok());
        pool.expect("Assert above should force this to be ok()")
    }

    #[tokio::test]
    async fn clear_workers_success() {
        let pool = get_sqlite_conn().await;
        let app = TauriApp::new(&pool).await;
    
        let app = app.clear_workers_collection().await;
        assert!(app.worker_store.list_worker().await.is_ok_and(|f| f.iter().count() == 0 ));
    }
}