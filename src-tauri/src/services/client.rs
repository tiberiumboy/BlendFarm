/*
    Developer blog:
    - Do some research on concurrent http downloader for transferring project files and blender from one client to another.
*/
use super::{
    message::{CmdMessage, Destination, NetMessage},
    server::MULTICAST_SOCK,
};
use crate::models::server_setting::ServerSetting;
use blender::blender::Manager;
use local_ip_address::local_ip;
use message_io::network::{Endpoint, Transport};
use message_io::node::{self, StoredNetEvent, StoredNodeEvent};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr};
#[cfg(target_family = "unix")]
use std::os::unix::fs::MetadataExt;
#[cfg(target_family = "windows")]
use std::os::windows::fs::MetadataExt;
use std::{net::SocketAddr, sync::mpsc, thread, time::Duration};

const INTERVAL_MS: u64 = 500;

/*
    In C - the best way to allow application API exposed for other application
    to rely on this information is through handler context.
    provide a pointer in heap of information stored to fetch information about
    network information and use the library implementation to perform action on
    those set of information.
*/
pub struct Client {
    tx: mpsc::Sender<CmdMessage>,
    // Is there a way for me to hold struct objects while performing a transfer task?
    // ^Yes, box heap the struct! See example - https://github.com/sigmaSd/LanChat/blob/master/net/src/lib.rs
}

// I wonder if it's possible to combine server/client code together to form some kind of intristic networking solution?

impl Client {
    pub fn new() -> Client {
        let (handler, listener) = node::split::<NetMessage>();
        let public_addr = SocketAddr::new(local_ip().unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST)), 0);
        let (_task, mut receiver) = listener.enqueue();

        // listen tcp
        if let Err(e) = handler.network().listen(Transport::FramedTcp, public_addr) {
            panic!("Unable to listen to tcp! \n{}", e);
        };

        // listen udp
        if let Err(e) = handler.network().listen(Transport::Udp, MULTICAST_SOCK) {
            println!("Unable to listen to udp! \n{}", e);
        }

        // connect udp
        let (udp_conn, _) = match handler.network().connect(Transport::Udp, MULTICAST_SOCK) {
            Ok(conn) => conn,
            Err(e) => panic!("Somethiing terrible happen! {e:?}"),
        };

        let (tx, rx) = mpsc::channel();
        // let tx_owned = tx.clone();
        thread::spawn(move || {
            // client should only have a connection to the server, maybe a connection to transfer files?
            let mut server: Option<Endpoint> = None;
            // let mut _current_job: Option<Job> = None;

            // this will help contain the job list I need info on.
            // Feature: would this be nice to load any previous known job list prior to running this client?
            // E.g. store in json file if client gets shutdown
            // let mut jobs: HashSet<Job> = HashSet::new();

            loop {
                std::thread::sleep(Duration::from_millis(INTERVAL_MS));
                if let Ok(msg) = rx.try_recv() {
                    match msg {
                        CmdMessage::AddPeer { .. } => {
                            // client should not have the ability to add peers.
                        }
                        // CmdMessage::SendJob(job) => {
                        //     // TODO: Move this code out?
                        //     // assume that we have the files already download and available, we should then run the job here?
                        //     let mut manager = Manager::load();
                        //     let blender = manager.fetch_blender(&job.version).unwrap();
                        //     let settings = server_setting::ServerSetting::load();
                        //     let args = Args::new(
                        //         job.project_file.file_path(),
                        //         settings.render_dir,
                        //         job.mode,
                        //     );
                        //     // eventually, I'd like to get to the point where I could render this?
                        //     println!("Rendering!");
                        //     let _receiver = blender.render(args);
                        //     while let Ok(status) = _receiver.recv() {
                        //         match status {
                        //             blender::models::status::Status::Idle => {
                        //                 println!("Blender[IDL]")
                        //             }
                        //             blender::models::status::Status::Running { status } => {
                        //                 println!("Blender[MSG]: {status}")
                        //             }
                        //             blender::models::status::Status::Log { status } => {
                        //                 println!("Blender[LOG]: {status}");
                        //             }
                        //             blender::models::status::Status::Warning { message } => {
                        //                 println!("Blender[WAR]: {message}");
                        //             }
                        //             blender::models::status::Status::Error(e) => {
                        //                 println!("Blender[ERR]: {e}");
                        //             }
                        //             // TODO: how do I check and see if I have any pending renders?
                        //             blender::models::status::Status::Completed { result } => {
                        //                 println!("Render completed! {:?}", result);
                        //                 // here I need to find a way to send the file back to the host
                        //                 // and tell it this is render image XXX for job XXX?

                        //                 // Ok we need to do two things.
                        //                 // one is we need to send the image back to the host
                        //                 // then two we need to let the host hey I'm done with this render image!
                        //                 // the reason for above is that we don't want the host to know we're done if we have another animation to render.
                        //                 // we should just send the stats information to let the user know their progress on this current node.
                        //                 // TODO: Find a way to get the server host? How? I thought I have this information somewhere?
                        //                 let active_server = match server {
                        //                     Some(server) => server,
                        //                     None => break,
                        //                 };

                        //                 let cmd = CmdMessage::SendFile(
                        //                     result,
                        //                     Destination::Target(active_server),
                        //                 );
                        //                 // once this is done, then we can go off and tell the render job, hey I'm done!
                        //                 // handler.network().send(cmd).unwrap();
                        //                 tx_owned.send(cmd).unwrap();

                        //                 break;
                        //             }
                        //         }
                        //     }

                        //     // notify the host that we're available.
                        //     if let Some(server) = server {
                        //         let completion = NetMessage::RequestJob.serialize();
                        //         handler.network().send(server, &completion);
                        //     }
                        // }
                        // function duplicated in server struct - may need to move this code block to a separate struct to handle network protocol between server/client
                        CmdMessage::SendFile(file_path, Destination::Target(target)) => {
                            // here the client is sending the file to either the server or client.
                            let mut file = File::open(&file_path).unwrap();
                            let file_name = file_path.file_name().unwrap().to_str().unwrap();
                            #[cfg(target_family = "windows")]
                            let size = file.metadata().unwrap().file_size() as usize;
                            #[cfg(target_family = "unix")]
                            let size = file.metadata().unwrap().size() as usize;
                            let mut data: Vec<u8> = Vec::with_capacity(size);
                            let bytes = file.read_to_end(&mut data).unwrap();
                            if bytes != size {
                                println!("Something wrong! buffer not the same size as file size!");
                            }
                            let msg = NetMessage::SendFile(file_name.to_owned(), data).serialize();
                            handler.network().send(target, &msg);
                        }
                        CmdMessage::SendFile(_, Destination::All) => {
                            println!("Unable to send files to all, can only send to server!");
                        }
                        CmdMessage::Ping => {
                            // send a ping to the network
                            println!("Received a ping request!");
                            handler.network().send(
                                udp_conn,
                                &NetMessage::Ping { server_addr: None }.serialize(),
                            );
                        }
                        CmdMessage::AskForBlender { version } => {
                            if let Some(conn) = server {
                                let msg = NetMessage::CheckForBlender {
                                    os: std::env::consts::OS.to_owned(),
                                    version,
                                    arch: std::env::consts::ARCH.to_owned(),
                                    caller: public_addr,
                                };
                                handler.network().send(conn, &msg.serialize());
                            }
                        }
                        CmdMessage::Exit => {
                            // Terminated signal received!
                            // should we also close the receiver?
                            handler.stop();
                            break;
                        }
                        CmdMessage::GetPeers => {
                            // do nothing, this command is reserve for the server only.
                        }
                    }
                }

                if let Some(StoredNodeEvent::Network(event)) = receiver.try_receive() {
                    match event {
                        StoredNetEvent::Connected(endpoint, _) => {
                            // we connected via udp channel!
                            if endpoint == udp_conn {
                                println!("Connected via UDP channel! [{}]", endpoint.addr());

                                // we then send out a ping signal on udp channel
                                handler.network().send(
                                    udp_conn,
                                    &NetMessage::Ping { server_addr: None }.serialize(),
                                );
                            }
                            // we connected via tcp channel!
                            else {
                                println!("Connected via TCP channel! [{}]", endpoint.addr());
                                server = Some(endpoint);
                                // TODO: should this be a default thing once client node is connected?
                                handler
                                    .network()
                                    .send(endpoint, &NetMessage::RequestJob.serialize());
                            }
                        }
                        StoredNetEvent::Accepted(endpoint, _) => {
                            // an tcp connection accepts the connection!
                            println!("Client was accepted to server! [{}]", endpoint.addr());
                            // self.server_endpoint = Some(endpoint);
                        }
                        StoredNetEvent::Message(endpoint, bytes) => {
                            let message = match NetMessage::deserialize(&bytes) {
                                Ok(msg) => msg,
                                Err(e) => {
                                    println!("unable to deserialize net message! \n{}", e);
                                    continue;
                                }
                            };

                            println!("Message received from [{}]!", endpoint.addr());

                            match message {
                                // server to client
                                // we received a ping signal from the server that accepted our ping signal.
                                // this means that either the server send out a broadcast signal to identify lost node connections on the network
                                NetMessage::Ping {
                                    server_addr: Some(socket),
                                } if server.is_none() => {
                                    match handler.network().connect(Transport::FramedTcp, socket) {
                                        Ok((endpoint, _)) => {
                                            server = Some(endpoint);
                                            println!("Connected to server! [{}]", endpoint.addr());
                                        }
                                        Err(e) => {
                                            println!("Error connecting to the server! \n{}", e);
                                        }
                                    }
                                }
                                NetMessage::Ping { .. } => {
                                    // ignore the ping signal from the client
                                }
                                // NetMessage::SendJob(job) => {
                                //     println!("Received a new job!\n{:?}", job);
                                // let msg = CmdMessage::SendJob(job);
                                // if let Err(e) = tx_owned.send(msg) {
                                //     println!("Fail to send job command internally!\n{e}");
                                // }
                                // current_job = Some(job);

                                // First let's check if we have the correct blender installation
                                // then check and see if we have the files?
                                // if !.project_file.file_path().exists() {
                                //     // here we will fetch the file path from the server
                                //     // but for now let's continue.
                                //     println!("Path does not exist!");
                                // }
                                /*

                                // run the blender() - this will take some time. Could implement async/thread?
                                match render_queue.run(1) {
                                    // returns frame and image path
                                    Ok(render_info) => {
                                        println!(
                                            "Render completed! Sending image to server! {:?}",
                                            render_info
                                        );

                                        let mut file_transfer = FileTransfer::new(
                                            render_info.path.clone(),
                                            endpoint,
                                        );

                                        // yeah gonna have to revisit this part...
                                        // file_transfer.transfer(&handler);
                                        // is there a way to convert mutable to immutable?

                                        // self.file_transfer = Some(file_transfer);
                                        // wonder if there's a way to say - hey I've completed my transfer,
                                        // please go and look in your download folder with this exact file name,
                                        // then proceed to your job manager to move out to output destination.
                                        // first notify the server that the job is completed and prepare to receive the file
                                        let msg = NetMessage::JobResult(render_info);
                                        handler.network().send(endpoint, &msg.ser());

                                        // let msg = Message::FileRequest(info);
                                        // self.send_to_target(self.server_endpoint, msg);
                                    }
                                    Err(e) => println!("Fail to render on client! {:?}", e),
                                }
                                */
                                // }
                                NetMessage::CheckForBlender {
                                    os,
                                    version,
                                    arch,
                                    caller,
                                } if os == std::env::consts::OS
                                    && arch == std::env::consts::ARCH =>
                                {
                                    println!("Received a blender check request from [{}]", caller);

                                    // here we will check and see if we have blender installed
                                    let manager = Manager::load();
                                    if manager
                                        .get_blenders()
                                        .iter()
                                        .any(|b| b.get_version().eq(&version))
                                    {
                                        // send a reply back!
                                        match handler
                                            .network()
                                            .connect(Transport::FramedTcp, caller)
                                        {
                                            Ok((endpoint, _)) => {
                                                handler.network().send(
                                                    endpoint,
                                                    &NetMessage::CanReceive(true).serialize(),
                                                );
                                            }
                                            Err(e) => {
                                                println!("Error connecting to the client! \n{}", e);
                                            }
                                        }
                                    }
                                    // let have_blender = check_blender(&os, &version, &arch);
                                    // self.contain_blender_response(endpoint, have_blender);
                                }
                                NetMessage::SendFile(file_name, data) => {
                                    // Problem - program crash if the file already exist -
                                    // need to save the file in temp location first, then move into the directory when completed.
                                    // if duplicated file exist - find the best mitigate plan? e.g. metadata comparison

                                    // TODO: Find a better way to check the directory and valdiate it exist.

                                    // it would be nice if we can get the server settings safe in here?
                                    // let server = ServerSetting::load();
                                    // let mut dst = server.blend_dir;
                                    // dst.push(&src.file_name().unwrap());
                                    // let name = file_name.to_str().unwrap().to_string();

                                    // if let Err(e) = std::fs::copy(&src, &dst) {
                                    //     // this may happen when the user moves computer (MacOS Transfer) - Temp storage do not exist because it has been renamed to something else.
                                    //     // TODO: validate temp storage exist before using it. - ServerSetting::load()
                                    //     println!("Unable to copy file from [{src:?}] to [{dst:?}]: {e}");
                                    // }

                                    let tmp = std::env::temp_dir().join(&file_name);

                                    if tmp.exists() {
                                        if let Err(e) = fs::remove_file(&tmp) {
                                            println!("Unable to delete existing file in tmp?\n{tmp:?}\n{e}");
                                        }
                                    }

                                    let mut file = File::create_new(&tmp).unwrap();
                                    if let Err(e) = file.write_all(&data) {
                                        println!("Fail to create new temp file! \n{e}");
                                    }

                                    // generate destination path.
                                    let server = ServerSetting::load();
                                    let output = server.blend_dir.join(file_name);
                                    if output.exists() {
                                        if let Err(e) = fs::remove_file(&output) {
                                            println!("Problem removing file!\n{e}");
                                        }
                                    }

                                    // if the file doesn't exist then we can simply move it to output path.
                                    if let Err(e) = fs::rename(&tmp, &output) {
                                        println!("Fail to move the file from temp [{tmp:?}] to destination [{output:?}]\n{e}");
                                    }

                                    println!(
                                        "Successfully received a file from {}",
                                        endpoint.addr()
                                    );
                                }
                                // client to client
                                _ => {
                                    println!(
                                        "Unhandled client message case condition for {:?}",
                                        message
                                    );
                                }
                            };
                        }
                        StoredNetEvent::Disconnected(endpoint) => {
                            // TODO: How can we initialize another listening job? We definitely don't want the user to go through the trouble of figuring out which machine has stopped.
                            // Disconnected was call when server was shut down
                            println!("Lost connection to host! [{}]", endpoint.addr());
                            server = None;
                            // in the case of this node disconnecting, I would like to auto renew the connection if possible.
                        }
                    }
                }
            }
        });

        Self { tx }
    }

    // TODO: find a way to set up invoking mechanism to auto ping out if we do not have any connection to the server
    /// Manually invoke the ping code (I.e. internet restore, Network restarted, interrupted, etc)
    #[allow(dead_code)]
    pub fn ping(&self) {
        println!("Sending ping command from client");
        self.tx.send(CmdMessage::Ping).unwrap();
    }

    // Call to the server and ask other node on the network if anyone have identical hardware machine that we can obtain blender from
    // If some time pass - we will begin downloading blender from the internet. Fail-safe
    // pub fn ask_for_blender(&self, version: Version) {
    //     self.tx.send(CmdMessage::AskForBlender { version }).unwrap();
    // }

    // Let's not worry about this for now...
    /*
    fn contain_blender_response(&self, endpoint: Endpoint, have_blender: bool) {
        if !have_blender {
            println!(
                "Client [{}] does not have the right version of blender installed",
                endpoint.addr()
            );
            return;
        }

        self.handler
            .network()
            .send(endpoint, &NetMessage::CanReceive(true).ser());
    }

    fn file_request(&mut self, endpoint: Endpoint, file_info: &FileInfo) {
        println!("name: {:?} | size: {}", file_info.path, file_info.size);
        let message = NetMessage::CanReceive(true);
        let data = bincode::serialize(&message).unwrap();
        self.handler.network().send(endpoint, &data);
        // TODO: Find a way to send file from one computer to another!
    }
    */
}

// TODO: I do need to implement a Drop method to handle threaded task. Making sure they're close is critical!
impl Drop for Client {
    fn drop(&mut self) {
        self.tx.send(CmdMessage::Exit).unwrap();
    }
}
