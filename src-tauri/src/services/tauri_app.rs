/* DEV Blog

    Issue: files provider are stored in memory, and do not recover after application restart.
        - mitigate this by using a persistent storage solution instead of memory storage.

    Issue: Cannot debug this application unless it is built completely. See if there's a way to run debug mode without building the app entirely.
*/

use super::{
    blend_farm::BlendFarm,
    data_store::{sqlite_job_store::SqliteJobStore, sqlite_worker_store::SqliteWorkerStore},
};
use crate::{
    domains::{job_store::{JobError, JobStore}, worker_store::WorkerStore},
    models::{
        app_state::AppState,
        computer_spec::ComputerSpec,
        job::{CreatedJobDto, JobEvent, JobId, NewJobDto},
        message::{Event, NetworkError},
        network::{NetworkController, NodeEvent, ProviderRule},
        server_setting::ServerSetting,
        task::Task,
        worker::Worker,
    },
    routes::{job::*, remote_render::*, settings::*, util::*, worker::*},
};
use async_std::task::sleep;
use blender::{blender::Blender, manager::Manager as BlenderManager, models::mode::RenderMode};
use futures::{
    SinkExt, StreamExt,
    channel::mpsc::{self, Sender},
};
use libp2p::PeerId;
use maud::html;
use semver::Version;
use sqlx::{Pool, Sqlite};
use std::{collections::HashMap, ops::Range, path::PathBuf, str::FromStr, time::Duration};
use tauri::{self, command, Url};
use tokio::{select, spawn, sync::Mutex};
use bitflags;

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

bitflags::bitflags! {
    #[derive(Debug, PartialEq)]
    pub struct QueryMode: u8 {
        const LOCAL = 0x1;
        const ONLINE = 0x2;
    }
}

#[derive(Debug, PartialEq)]
pub enum Origin {
    Local(PathBuf),
    Online(Url),
}

#[derive(Debug)]
pub struct BlenderQuery {
    pub version: Version,
    pub origin: Origin,
}

impl BlenderQuery {
    pub fn is_install_locally(&self) -> bool {
        match self.origin {
            Origin::Local(_) => true,
            _ => false,
        }
    }

    pub fn link(&self) -> String {
        match &self.origin {
            // TODO: Find a way to resolve expect()
            Origin::Local(path) => path.to_str().expect("Should be valid").to_owned(),
            Origin::Online(url) => url.to_string().to_owned()
        }
    }
}

#[derive(Debug)]
pub enum BlenderAction {
    Add(PathBuf),
    List(Sender<Option<Vec<BlenderQuery>>>, QueryMode),
    Get(Version, Sender<Option<Blender>>),
    Disconnect(Blender), // detach links associated with file path, but does not delete local installation!
    Remove(Blender), // deletes local installation of blender, use it as last resort option. (E.g. force cache clear/reinstall/ corrupted copy)
}

impl PartialEq for BlenderAction {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Add(l0), Self::Add(r0)) => l0 == r0,
            (Self::List(.., l0), Self::List(.., r0)) => l0 == r0,
            (Self::Get(l0, ..), Self::Get(r0, ..)) => l0 == r0,
            (Self::Disconnect(l0), Self::Disconnect(r0)) => l0 == r0,
            (Self::Remove(l0), Self::Remove(r0)) => l0 == r0,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub enum JobAction {
    Find(JobId, Sender<Option<CreatedJobDto>>),
    Update(CreatedJobDto),
    Create(NewJobDto, Sender<Result<CreatedJobDto, JobError>>),
    Kill(JobId),
    All(Sender<Option<Vec<CreatedJobDto>>>),
    // we will ask all of the node on the network if there's any completed job list.
    // The node will advertise their collection of completed job
    // the host will be responsible to compare with the current output files and 
    // see if there's any missing job. If there is missing frame then 
    // we will ask to fetch for that completed image back
    AskForCompletedList(JobId), 
    Advertise(JobId),
}

impl PartialEq for JobAction {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Find(l0, ..), Self::Find(r0, ..)) => l0 == r0,
            (Self::Update(l0), Self::Update(r0)) => l0.id == r0.id,
            (Self::Create(l0, ..), Self::Create(r0,.. )) => l0 == r0,
            (Self::Kill(l0), Self::Kill(r0)) => l0 == r0,
            (Self::All(..), Self::All(..)) => true,
            (Self::AskForCompletedList(l0), Self::AskForCompletedList(r0)) => l0 == r0,
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

pub struct TauriApp {
    // I need the peer's address? I don't think I need the PeerId, but will hold onto it just in case.
    // we may ultimately change this to rely on the computer name instead of PeerId?
    peers: HashMap<PeerId, ComputerSpec>,
    worker_store: SqliteWorkerStore,
    job_store: SqliteJobStore,
    settings: ServerSetting,
    manager: BlenderManager,
}

#[command]
pub fn index() -> String {
    html! (
        div {
            div class="sidebar" {
                nav {
                    ul class="nav-menu-items" {
                        
                        // li key="manager" class="nav-bar" tauri-invoke="remote_render_page" hx-target=(format!("#{WORKPLACE}")) {
                        //     span { "Remote Render" }
                        // };

                        li key="setting" class="nav-bar" tauri-invoke="setting_page" hx-target=(format!("#{WORKPLACE}")) {
                            span { "Setting" }
                        };
                    };
                };
                div {
                    h3 { "Jobs" }

                    button tauri-invoke="open_dialog_for_blend_file" hx-target="body" hx-swap="beforeend" {
                        "Import"
                    };

                    // Is there a way to select the first item on the list by default?
                    // TODO: Take a look into hx-swap-oob on how we can refresh when a record is deleted or added
                    div class="group" id="joblist" tauri-invoke="list_jobs" hx-trigger="load" hx-target="this";
                }

                // div {
                //     h2 { "Computer Nodes" };
                //     // hx-trigger="every 10s" - omitting this as this was spamming console log
                //     div class="group" id="workers" tauri-invoke="list_workers" hx-target="this" {};
                // };
            };

        }
        main id=(WORKPLACE);
    ).0
}

impl TauriApp {
    // Clear worker database before usage!
    pub async fn clear_workers_collection(mut self) -> Self {
        if let Err(e) = self.worker_store.clear_worker().await {
            eprintln!("Error clearing worker database! {e:?}");
        }
        self
    }

    pub async fn new(pool: &Pool<Sqlite>) -> Self {
        Self {
            peers: Default::default(),
            worker_store: SqliteWorkerStore::new(pool.clone()),
            job_store: SqliteJobStore::new(pool.clone()),
            settings: ServerSetting::load(),
            manager: BlenderManager::load(),
        }
    }

    // Create a builder to make Tauri application
    // Let's just use the controller in here anyway.
    pub fn init_tauri_plugins<R: tauri::Runtime>(
        builder: tauri::Builder<R>
    ) -> tauri::Builder<R> {
        builder
            .plugin(tauri_plugin_cli::init())
            .plugin(tauri_plugin_os::init())
            .plugin(tauri_plugin_fs::init())
            .plugin(tauri_plugin_persisted_scope::init())
            .plugin(tauri_plugin_shell::init())
            .plugin(tauri_plugin_dialog::init())
    }

    // This design implement doesn't fit the concept of decentralized network situation setup.
    // We shouldn't have to rely on finding node availability, instead other node should ping out to other node and offer help instead of relying the host to do the work.
    /*
    async fn get_idle_peers(&self) -> String {
        // see comment above, this method is no longer in use.
    }
    */

    // The idea here is to generate new task based on job creation.
    // TODO: Explain the expect behaviour for this method before reference it.
    #[allow(dead_code)]
    fn generate_tasks(job: &CreatedJobDto, chunks: i32) -> Vec<Task> {
        // mode may be removed soon, we'll see?
        let (time_start, time_end) = match job.item.get_mode() {
            RenderMode::Animation(anim) => (anim.start, anim.end),
            RenderMode::Frame(frame) => (frame.clone(), frame.clone()),
        };

        // What if it's in the negative? e.g. [-200, 2 ] ? would this result to -180 and what happen to the equation?
        // ^^^^ TODO: This is a good example for unit test!
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

            // TODO: Find a way to handle this error. 
            // It should only error if we don't have permission to temp cache storage location
            let task = Task::from(
                job.clone(),
                range,
            ).expect("Should be able to create task!");
            tasks.push(task);
        }

        tasks
    }

    async fn handle_job_command(&mut self, job_action: JobAction, client: &mut NetworkController) {
        match job_action {
            JobAction::Find(job_id, mut sender) => {
                let result = self.job_store.get_job(&job_id).await;
                match result {
                    Ok(record) => {
                        if let Err(e) = sender.send(record).await {
                            eprintln!("Unable to get a job!: {e:?}");
                        }
                    }
                    Err(e) => eprintln!("Job store reported an error: {e:?}"),
                };
            }
            JobAction::Update(job) => {
                // as long as the uuid exist in the database, we should be fine to update the job entry.
                let result = self.job_store.update_job(job).await;
                if let Err(e) = result {
                    eprintln!("Fail to update job! {e:?}");
                }
            }
            JobAction::Create(job, mut sender) => {
                let result = self.job_store.add_job(job).await;

                let res = match result {
                    Ok(job) => sender.send(Ok(job)).await,
                    Err(e) => sender.send(Err(JobError::DatabaseError(e.to_string()))).await
                };

                if let Err(e) = res {
                    eprintln!("Fail to call sender from jobaction::create! {e:?}");
                }
            }
            JobAction::Kill(job_id) => {
                if let Err(e) = self.job_store.delete_job(&job_id).await {
                    eprintln!("Receiver/sender should not be dropped! {e:?}");
                }
                let (sender, mut receiver) = mpsc::channel(1);
                client.send_job_event(JobEvent::Remove(job_id), sender).await;

                if let Err(e) = receiver.select_next_some().await {
                    eprintln!("Fail to send job event! {e:?}");
                    sleep(Duration::from_secs(5u64)).await;
                }
            }
            JobAction::AskForCompletedList(job_id) => {
                // here we will try and send out network node asking for any available client for the list of completed frame images.
                let (sender, mut receiver ) = mpsc::channel(1);
                let event = JobEvent::AskForCompletedJobFrameList(job_id);
                client.send_job_event(event, sender).await;
                if let Err(e) = receiver.select_next_some().await {
                    eprintln!("Fail to send job event! {e:?}");
                    sleep(Duration::from_secs(5u64)).await;
                }
            }
            JobAction::All(mut sender) => {
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
                    }
                    Err(e) => {
                        eprintln!("Unable to send list of jobs: {e:?}");
                        None
                    }
                };

                if let Err(e) = sender.send(result).await {
                    eprintln!("Fail to send data back! {e:?}");
                }
            }
            JobAction::Advertise(job_id) =>
            // Here we will simply add the job to the database, and let client poll them!
            {
                let result = match self.job_store.get_job(&job_id).await {
                    Ok(job) => job,
                    Err(e) => {
                        eprintln!("No Job record found! Skipping! {e:?}");
                        return ();
                    }
                };

                // first make the file available on the network
                if let Some(job) = result {
                    let _file_name = job.item.get_project_path().file_name().unwrap(); // this is &OsStr
                    let path = job.item.get_project_path().clone();
    
                    // Once job is initiated, we need to be able to provide the files for network distribution.
                    let _provider = ProviderRule::Default(path.to_path_buf());
                }

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
            }
        }
    }

    async fn handle_blender_command(&mut self, blender_action: BlenderAction) {
        match blender_action {
            BlenderAction::Add(_blender) => {
                todo!("impl adding blender?");
            }
            BlenderAction::List(mut sender, flags) => {
                let mut versions = Vec::new();
                
                if flags.contains(QueryMode::LOCAL) {

                    let mut localblenders = self.manager.get_blenders().iter().map(|b| BlenderQuery {
                        version: b.get_version().to_owned(), 
                        origin: Origin::Local(b.get_executable().into())
                    }).collect::<Vec<BlenderQuery>>(); 
                    versions.append(&mut localblenders);
                }
            
                // then display the rest of the download list
                // TODO: Figure out why fetch_download_list() takes awhile to query the data. 
                // I expect the cache should fetch the info and provide that information rather than querying the internet 
                // everytime this function is called.
                if flags.contains(QueryMode::ONLINE) {
                    if let Some(downloads) = self.manager.fetch_download_list() {
                        let mut item = downloads
                        .iter()
                        .map(|d| BlenderQuery { 
                            version: d.get_version().clone(), 
                            origin: Origin::Online(d.get_url().clone()) 
                        })
                        .collect::<Vec<BlenderQuery>>();
                        versions.append(&mut item);
                    }; 
                }
                
            
                // send the collective list result back
                if let Err(e) = sender.send(Some(versions)).await {
                    eprintln!("Fail to send back list of blenders to caller! {e:?}");
                }
            }
            BlenderAction::Get(version, mut sender) => {
                let result = self.manager.fetch_blender(&version);
                match result {
                    Ok(blender) => {
                        if let Err(e) = sender.send(Some(blender)).await {
                            eprintln!("Fail to send result back to caller! {e:?}");
                        }
                    }
                    Err(e) => {
                        eprintln!("Fail to fetch blender! {e:?}");
                        if let Err(e) = sender.send(None).await {
                            eprintln!("Fail to send result back to caller! {e:?}");
                        }
                    }
                };
            }
            // severe connection - remove the entry from database, but do not touch the installation
            BlenderAction::Disconnect(blender) => {
                self.manager.remove_blender(&blender);
            },
            // uninstall blender from local machine
            BlenderAction::Remove(blender) => {
                self.manager.delete_blender(&blender);
            },
        }
    }

    async fn handle_worker_command(&mut self, worker_action: WorkerAction) {
        match worker_action {
            WorkerAction::Get(peer_id, mut sender) => {
                let result = sender
                    .send(self.worker_store.get_worker(&peer_id).await)
                    .await;
                if let Err(e) = result {
                    eprintln!("Unable to get worker!: {e:?}");
                }
            }
            WorkerAction::List(mut sender) => {
                let result = sender
                    .send(self.worker_store.list_worker().await.ok())
                    .await;
                if let Err(e) = result {
                    eprintln!("Unable to send list of workers: {e:?}");
                }
            }
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
            UiCommand::Settings(setting_action) => {
                self.handle_setting_command(setting_action).await
            }
            UiCommand::Job(job_action) => self.handle_job_command(job_action, client).await,
            UiCommand::Worker(worker_action) => self.handle_worker_command(worker_action).await,
            UiCommand::UploadFile(path) => {
                // this is design to notify the network controller to start advertise provided file path
                let provider = ProviderRule::Default(path);
                if let Err(e) = client.start_providing(&provider).await {
                    eprintln!("Network issue on providing file! {e:?}");
                }
            }
        }
    }

    // commands received from network
    async fn handle_net_event(&mut self, client: &mut NetworkController, event: Event) {
        match event {
            Event::NodeStatus(node_status) => match node_status {
                NodeEvent::Connected(peer_id_string, spec) => {
                     
                        let peer_id =
                            PeerId::from_str(&peer_id_string).expect("Peer id should be valid");
                        let worker = Worker::new(peer_id.clone(), spec.clone());
                        // append new worker to database store
                        if let Err(e) = self.worker_store.add_worker(worker).await {
                            eprintln!("Error adding worker to database! {e:?}");
                        }
    
                        // self.peers.insert(peer_id, spec);
                        // let handle = app_handle.write().await;
                        // emit a signal to query the data.
                        // TODO: See how this can be done: https://github.com/ChristianPavilonis/tauri-htmx-extension
                        // let _ = handle.emit("worker_update");
                    },
                // concerning - this String could be anything?
                // TODO: Find a better way to get around this.
                NodeEvent::Disconnected { peer_id, reason } => {
                    if let Some(msg) = reason {
                        eprintln!("Node disconnected with reason!\n {msg}");
                    }

                    // So the main issue is that there's no way to identify by the machine id?
                    let peer_id =
                        PeerId::from_str(&peer_id).expect("Received invalid peer_id string!");

                    // probably best to mark the node "inactive" instead?
                    if let Err(e) = self.worker_store.delete_worker(&peer_id).await {
                        eprintln!("Error deleting worker from database! {e:?}");
                    }

                    self.peers.remove(&peer_id);
                }
                // this is the same as saying down in the garbage disposal. Anything goes here. Do not trust data source here!
                NodeEvent::BlenderStatus(blend_event) => {
                    println!("Blender Status Received: {blend_event:?}")
                }
            },

            // let me figure out what's going on here.
            // a network sent us a inbound request - reply back with the file data in channel.
            // yeah I wonder why we can't move this inside network class?
            Event::InboundRequest { request, channel } => {
                self.handle_inbound_request(client, request, channel).await;
            }

            Event::JobUpdate(job_event) => match job_event {
                // when we receive a completed image, send a notification to the host and update job index to obtain the latest render image.
                JobEvent::AskForCompletedJobFrameList(_)  => {
                    // this is reserved for the host side of the app to send out. We do not process this data here.
                    // only client should receive this notification, host will ignore this.
                }
                JobEvent::ImageCompletedList { job_id, files } => {
                    // first thing first, check and see if this job id matches what we have in our database.
                    // if it doesn't then we ignore this request and move on.
                    let result = self.job_store.get_job(&job_id).await;
                    
                    if result.is_err() {
                        return; // stop here. do not proceed forward. We do not care.
                    }
                    
                    // not that we have the job, we need to fetch for our existing files that we have completed
                    // We received a list of files from the client. We will run and compare this list to our local machine
                    // let local = 
                    
                    // if we do not have the file locally, we will ask for the image from the provided node.
                    // In this case, we do not care who have the node, we will send out a signal stating I need this file.
                    // the node that receive the signal will message back.
                    
                    for file in files {
                      println!("file: {file}");  
                    };
                }
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
                    // look into my jobs and see what jobs are available to send for remote renders
                    // How do I fetch a new task for the workers to consume?
                    let jobs = self.job_store.list_all().await.expect("Should have jobs?");
                    let job = jobs.first().unwrap().clone();
                    let task = job.item.generate_task(job.id);
                    // how do I reply back for this task then?
                }
                // this will soon go away
                JobEvent::Failed(msg) => {
                    eprintln!("Job failed! {msg}");
                }
                JobEvent::Remove(_) => {
                    // Should I do anything on the manager side? Shouldn't matter at this point?
                }
            },
            _ => {} // println!("[TauriApp]: {:?}", event),
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

        let app_state = AppState::new(event);
        let mut_app_state = Mutex::new(app_state);

        // we send the sender to the tauri builder - which will send commands to "from_ui".
        let app = Self::init_tauri_plugins(tauri::Builder::default())
            .invoke_handler(tauri::generate_handler![
                index,
                open_path,
                open_dir,
                select_directory,
                select_file,
                create_job,
                delete_job,
                get_job_detail,
                setting_page,
                edit_settings,
                get_settings,
                update_settings,
                open_dialog_for_blend_file,
                available_versions,
                list_workers,
                list_jobs,
                get_worker,
                update_output_field,
                add_blender_installation,
                list_blender_installed,
                disconnect_blender_installation,
                uninstall_blender,
                delete_blender,
                fetch_blender_installation,
            ])
            .manage(mut_app_state)
            .build(tauri::generate_context!("tauri.conf.json"))
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
    use super::*;
    use crate::config_sqlite_db;

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
        assert!(
            app.worker_store
                .list_worker()
                .await
                .is_ok_and(|f| f.iter().count() == 0)
        );
    }

    // todo: identify other part of this code that I can run unit test and list out potential edge cases
}
