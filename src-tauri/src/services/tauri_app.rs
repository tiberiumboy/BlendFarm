/* DEV Blog

    Issue: files provider are stored in memory, and do not recover after application restart.
        - mitigate this by using a persistent storage solution instead of memory storage.

    Issue: Cannot debug this application unless it is built completely. See if there's a way to run debug mode without building the app entirely.
*/

use super::{
    blend_farm::BlendFarm,
    data_store::{sqlite_job_store::SqliteJobStore, sqlite_worker_store::SqliteWorkerStore},
};
use crate::network::provider_rule::ProviderRule;
use crate::network::{controller::Controller as NetworkController, message::Event};
use crate::services::blend_farm::BlendFarmError;
use crate::services::server::ServerEvent;
use crate::{
    domains::{
        job_store::{JobError, JobStore},
        worker_store::WorkerStore,
    },
    models::{
        app_state::AppState,
        blender_action::BlenderAction,
        computer_spec::ComputerSpec,
        job::{CreatedJobDto, JobAction, JobEvent},
        server_setting::ServerSetting,
        setting_action::SettingsAction,
        ticket::Ticket,
        worker::Worker,
    },
    routes::{index::*, job::*, remote_render::*, settings::*, util::*, worker::*},
};
use async_trait::async_trait;
use bitflags;
use blender::{
    blend_file::BlendFile, manager::Manager as BlenderManager, models::mode::RenderMode,
};
use futures::{
    SinkExt, StreamExt,
    channel::mpsc::{self, Sender},
};
use libp2p::PeerId;
use semver::Version;
use sqlx::{Pool, Sqlite};
use std::{collections::HashMap, path::PathBuf, str::FromStr};
use tauri::{self, Url};
use tokio::sync::mpsc::Receiver;
use tokio::{select, spawn, sync::Mutex};

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
            Origin::Online(url) => url.to_string().to_owned(),
        }
    }

    pub fn parent_dir(&self) -> String {
        match &self.origin {
            Origin::Local(path) => path
                .parent()
                .and_then(|f| Some(f.as_os_str().to_str().unwrap().to_owned()))
                .unwrap_or_else(|| "".to_owned()),
            Origin::Online(_) => "".to_owned(),
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

impl TauriApp {
    // Clear worker database before usage!
    pub async fn clear_workers_collection(mut self) -> Self {
        if let Err(e) = self.worker_store.clear_worker().await {
            eprintln!("Error clearing worker database! {e:?}");
        }
        self
    }

    pub fn new(manager: BlenderManager, pool: &Pool<Sqlite>) -> Self {
        Self {
            peers: Default::default(),
            worker_store: SqliteWorkerStore::new(pool.clone()),
            job_store: SqliteJobStore::new(pool.clone()),
            settings: ServerSetting::load(),
            manager,
        }
    }

    // Create a builder to make Tauri application
    // Let's just use the controller in here anyway.
    pub fn init_tauri_plugins<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
        builder
            .plugin(tauri_plugin_cli::init())
            .plugin(tauri_plugin_os::init())
            .plugin(tauri_plugin_fs::init())
            .plugin(tauri_plugin_persisted_scope::init())
            .plugin(tauri_plugin_shell::init())
            .plugin(tauri_plugin_dialog::init())
    }

    // The idea here is to generate new task based on job creation.
    // TODO: Explain the expect behaviour for this method before reference it.
    #[allow(dead_code)]
    fn generate_tasks(job: &CreatedJobDto, chunks: i32) -> Vec<Ticket> {
        // mode may be removed soon, we'll see?
        let (time_start, time_end) = match AsRef::<RenderMode>::as_ref(&job.item) {
            RenderMode::Animation { start, end } => (start, end),
            RenderMode::Frame(frame) => (frame, frame),
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
                _ => time_end.to_owned(),
            };

            // TODO: Find a way to handle this error.
            // It should only error if we don't have permission to temp cache storage location
            let task =
                Ticket::from(job.clone(), start, end).expect("Should be able to create task!");
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
                            eprintln!("unable to send record back! \n{e:?}");
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

                match result {
                    Ok(job) => {
                        sender.send(Ok(job)).await.expect("Should not drop");
                    }
                    Err(e) => {
                        sender
                            .send(Err(JobError::DatabaseError(e.to_string())))
                            .await
                            .expect("Should not drop");
                    }
                };
            }
            JobAction::Kill(job_id) => {
                if let Err(e) = self.job_store.delete_job(&job_id).await {
                    eprintln!("Receiver/sender should not be dropped! {e:?}");
                }
                let server_event = ServerEvent::RemoveJob(job_id);
                client.send_broadcast_message(server_event).await;
            }
            // TODO: Figure out how we can handle/process this command. How does this get sent out? Do we ask when the user loads the job information or request an update?
            JobAction::AskForCompletedList(job_id) => {
                // How do I send out a broadcast signal, but don't send it to myself? (Exclude loopback messages?)
                let server_event = ServerEvent::RequestJobInfo(job_id);
                client.send_broadcast_message(server_event).await;
            }
            JobAction::All(mut sender) => {
                /*
                    There's something wrong with this datastructure.
                    On first call, this command works as expected,
                    however additional call afterward does not let this function continue or invoke?
                    I must be waiting for something here?
                */
                // TODO: Consider looking into using Iter() mutations.
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

            // Nothing is calling this yet???
            // this seems to be a server thing?
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
                    let project_file: &BlendFile = job.item.as_ref();
                    let file_name = project_file
                        .to_path()
                        .file_name()
                        .expect("Must have a valid blender file name!"); // this is &OsStr
                    let path: &PathBuf = job.item.as_ref();

                    println!("Reached to this point of code {file_name:?}");

                    // Once job is initiated, we need to be able to provide the files for network distribution.
                    let _provider = ProviderRule::Default(path.to_path_buf());
                    // this is where I'm confused?
                    // if let Err(e) = client.start_providing(&provider).await {
                    //     eprintln!("Fail to provide file! {e:?}");
                    //     return;
                    // }

                    // let tasks = Self::generate_tasks(
                    //     &job,
                    //     MAX_FRAME_CHUNK_SIZE
                    //     );

                    // // so here's the culprit. We're waiting for a peer to become idle and inactive waiting for the next job
                    // for task in tasks {
                    //     // problem here - I'm getting one client to do all of the rendering jobs, not the inactive one.
                    //     // Perform a round-robin selection instead.

                    //     println!("Sending task to {:?} \nRange( {} - {} )\n", &host, &task.range.start, &task.range.end);
                    //     client.send_job_event(Some(host.clone()), JobEvent::Render(task)).await;
                    // }
                }
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
                    let mut localblenders = self
                        .manager
                        .get_blenders()
                        .iter()
                        .map(|b| BlenderQuery {
                            version: b.get_version().to_owned(),
                            origin: Origin::Local(b.get_executable().into()),
                        })
                        .collect::<Vec<BlenderQuery>>();
                    versions.append(&mut localblenders);
                }

                // then display the rest of the download list
                // TODO: Figure out why fetch_download_list() takes awhile to query the data.
                // I expect the cache should fetch the info and provide that information rather than querying the internet
                // everytime this function is called.
                if flags.contains(QueryMode::ONLINE) {
                    let mut item = self.manager.get_online_version().iter().fold(
                        Vec::new(),
                        |mut map, (url, version)| {
                            let item = BlenderQuery {
                                version: version.clone(),
                                origin: Origin::Online(url.clone()),
                            };
                            map.push(item);
                            map
                        },
                    );
                    versions.append(&mut item);
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
                        let _ = sender.send(None);
                    }
                };
            }
            // severe connection - remove the entry from database, but do not touch the installation
            BlenderAction::Disconnect(blender) => {
                if let Err(e) = self.manager.remove_blender(&blender) {
                    eprintln!("Unable to disconnect blender: {e:?}");
                }
            }
            // uninstall blender from local machine
            BlenderAction::Remove(_blender) => {
                todo!("Need to do some unit test before you can use this feature...");
                // self.manager.delete_blender(&blender);
            }
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
    #[allow(dead_code)]
    async fn handle_net_event(&mut self, client: &mut NetworkController, event: Event) {
        match event {
            // A node was recently discovered from the network.
            Event::Discovered(..) => {
                // Here should try to join the topic hash before sending message out in case it doesn't work?
                let multiaddr = client.multiaddr.clone();
                let spec = ComputerSpec::new();
                // We replied back to the discovered node "Hello, this is my specs, so call me maybe?"
                let server_status = ServerEvent::Online(multiaddr, spec);
                client.send_broadcast_message(server_status).await
            }
            Event::InboundRequest { .. } => todo!(),
            // Listen to what the server update are happening on the network.
            Event::ServerStatus(event) => println!("Picked up Server Status: {event:?}"),
            Event::JobUpdate(..) => todo!(),
            Event::ReceivedFileData(..) => todo!(),
        }
    }
}

#[async_trait]
impl BlendFarm for TauriApp {
    async fn run(
        mut self,
        mut client: NetworkController,
        mut event_receiver: Receiver<Event>,
    ) -> Result<(), BlendFarmError> {
        // this channel is used to send command to the network, and receive network notification back.
        let (event, mut command) = mpsc::channel(32);

        let app_state = AppState::new(event);
        let mut_app_state = Mutex::new(app_state);

        // at the start of this program, I need to broadcast existing project file before the rest of the command hooks.
        // This way, any job pending would have the file already available to distribute across the network.

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
                install_from_internet,
                list_blender_installed,
                disconnect_blender_installation,
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

                    Some(net_event) = event_receiver.recv() => match net_event {
                        // TODO: We have handle_net_event() class, why aren't we using this?

                        Event::ServerStatus(server_status) => match server_status {
                            // ServerEvent::Hello(peer_id_string, spec) => {
                            //     // a new node acknowledges your activity. Revealing available server on the network.
                            //     // this node now listens to you, and has provided info to communicate back
                            //     let peer_id =
                            //         PeerId::from_str(&peer_id_string).expect("Peer id should be valid");

                            //     // We'll tag this node as a worker.
                            //     let worker = Worker::new(peer_id.clone(), spec.clone());

                            //     // append new worker to database store
                            //     if let Err(e) = self.worker_store.add_worker(worker).await {
                            //         eprintln!("Error adding worker to database! {e:?}");
                            //     }

                            //     println!("New worker added!");
                            //     self.peers.insert(peer_id, spec);

                            //     // let handle = app_handle.write().await;
                            //     // emit a signal to query the data.
                            //     // TODO: See how this can be done: https://github.com/ChristianPavilonis/tauri-htmx-extension
                            //     // let _ = handle.emit("worker_update");
                            // }
                            ServerEvent::Online(peer_id_str, spec ) => {
                                // discovered a node.
                                let name = spec.host;
                                // let peer_id = PeerId::from_str(&peer_id_str).expect("Received invalid peer_id string!");
                                println!("[{peer_id_str}] {name} is online.");
                            },

                            ServerEvent::NewTickets(peer_id_str) => {
                                // This node have new tickets available
                                // should we send out and request tickets?
                                // should tauri app cares?
                                println!("[{peer_id_str}] have new tickets available!");
                            },
                            ServerEvent::RequestTicket => {
                                // this node is requesting new tickets
                                println!("A node is idle and asking for new tickets");
                                // How do I check my job and see if I have any pending tickets/pending jobs to work on?
                                let new_job = match self.job_store.list_all().await {
                                    Ok(list) => list.iter().fold(None, |result, item| {
                                        if result.is_some() {
                                            return result
                                        }
                                        
                                        // now how do I know if the job is completed or not?
                                        if item
                                        
                                        Some(item)
                                    }),
                                    _ => return ()
                                };
                                

                            },
                            // which node?
                            ServerEvent::Rendering(uuid) => {
                                // we received a node update that they're now rendering this uuid.
                                println!("A node is working on {uuid}!");
                            },
                            ServerEvent::RequestJobInfo(job_id) => {
                                println!("A node is requesting job information that matches id {job_id}");
                                // a node is asking for job information that matches this target id
                            },
                            ServerEvent::RemoveJob(job_id) => {
                                // received a signal to remove target job id.
                                println!("Received orders to remove job that matches id {job_id}");
                            },
                            // concerning - this String could be anything?
                            // TODO: Find a better way to get around this.
                            ServerEvent::Disconnected { peer_id, reason } => {
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
                            ServerEvent::BlenderStatus(blend_event) => {
                                println!("Blender Status Received: {blend_event:?}")
                            }
                        },

                        // let me figure out what's going on here.
                        // a network sent us a inbound request - reply back with the file data in channel.
                        // yeah I wonder why we can't move this inside network class?
                        Event::InboundRequest { request, channel } => {
                            Self::handle_inbound_request(&client, request, channel).await;
                        }

                        Event::JobUpdate(job_event) => match job_event {
                            // when we receive a completed image, send a notification to the host and update job index to obtain the latest render image.
                            JobEvent::AskForCompletedJobFrameList(_) => {
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
                                }
                            }
                            // we received a job event that a node have finish rendering an image.
                            // We now need to make sure our output destination exist and valid.
                            // Afterward, we should try to fetch the file from that caller.
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

                                /*  send update to ui
                                let handle = app_handle.write().await;
                                if let Err(e) = handle.emit(
                                    "frame_update",
                                    FrameUpdatePayload {
                                        id,
                                        frame,
                                        file_name: file_name.clone(),
                                    },
                                ) {
                                    eprintln!("Unable to send emit to app handler\n{e:?}");
                                }
                                */

                                // Fetch the completed image file from the network
                                match client.get_file_from_peers(&file_name, &destination).await {
                                    Ok(file) => {
                                        println!("File stored at {file:?}");
                                        // let handle = app_handle.write().await;
                                        // if let Err(e) = handle.emit("job_image_complete", (job_id, frame, file)) {
                                        //     eprintln!("Fail to publish image completion emit to front end! {e:?}");
                                        // }
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to fetch the file from peers!\n{:?}", e);
                                    }
                                }
                            }
                            // when a task is complete, check the poll for next available job queue?
                            JobEvent::TicketComplete(job_id, frame) => {
                                println!("A node have completed frame {frame} for job id {job_id}");
                                // I don't understand why I got the frame?
                            }

                            // TODO: how do we handle error from node? What kind of errors are we expecting here and what can the host do about it?
                            JobEvent::Error(job_error) => {
                                todo!("See how this can be replicated? {job_error:?}")
                            }

                            // send a render job
                            JobEvent::Render(..) => {
                                // if we have a local client up and running, we should just communicate it directly. This will help setup the output correctly.
                                // TODO: Host should try to communicate local client
                                println!(
                                    "Host received a Render Job - Contact client and provide info about this job. Read on how Rust micromange services?"
                                );
                            }
                            // Not in used?
                            JobEvent::RequestTask => {
                                // a node is requesting task.
                                todo!("Where is this being called from? I tried looking up reference and found this to be the only place used");
                                // let jobs = self.job_store.list_all().await.expect("Should have jobs?");
                                // if let Some(job) = jobs.first() {
                                //     // how do I reply back for this task then?
                                //     // use the peer_id_string.
                                //     match job.item.clone().generate_task(job.id) {
                                //         Some(task) => {
                                //             let event = JobEvent::Render(peer_id_str, task);
                                //             client.send_job_event(event).await;
                                //         }
                                //         None => return,
                                //     }
                                // }
                            }
                            // this will soon go away
                            JobEvent::Failed(msg) => {
                                eprintln!("Job failed! {msg}");
                            }
                            JobEvent::Remove(_) => {
                                // Should I do anything on the manager side? Shouldn't matter at this point?
                            }
                        },
                        Event::Discovered(..) => {
                            // from this level, we have discovered other potential client on the network.
                            // at this level, we do absolutely nothing. We only respond to client incoming request.
                        },
                        e => println!("Unhandled Network Event {e:?}")
                    }
                }
            }
        });

        app.run(|_, _| {});
        Ok(())
    }
}

#[cfg(test)]
mod test {
    // use blender::models::blender_config::BlenderConfig;

    use super::*;
    use crate::{config_sqlite_db, constant::DATABASE_FILE_NAME};
    // use async_trait::async_trait;

    async fn get_sqlite_conn() -> Pool<Sqlite> {
        let pool = config_sqlite_db(DATABASE_FILE_NAME).await;
        assert!(pool.is_ok());
        pool.expect("Assert above should force this to be ok()")
    }

    // async fn get_mockup_config() -> BlenderConfig {
    //     todo!("Implement a mock up unit test for this blender config");
    // }

    async fn get_mockup_manager() -> BlenderManager {
        todo!("Implement a mock up blender manager");
    }

    #[tokio::test]
    async fn clear_workers_success() {
        let pool = get_sqlite_conn().await;
        // let config = get_mockup_config().await;
        let manager = get_mockup_manager().await;
        let app = TauriApp::new(manager, &pool);

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
