use super::message::{Message, Signal};
use message_io::{network::Endpoint, node::NodeHandler};
use std::{fs::File, io::Read, path::PathBuf, time::Duration};

const CHUNK_SIZE: usize = 65536;

pub struct FileTransfer {
    pub file_path: PathBuf,
    pub file: Option<File>,
    pub size: usize,
    pub destination: Endpoint,
    pub file_bytes_sent: usize,
}

impl FileTransfer {
    pub fn new(file_path: PathBuf, size: usize, destination: Endpoint) -> Self {
        let file = File::open(&file_path).ok();
        FileTransfer {
            file_path,
            file,
            size,
            destination,
            file_bytes_sent: 0,
        }
    }

    pub fn transfer(&mut self, handler: &NodeHandler<Signal>) -> Option<usize> {
        match &mut self.file {
            Some(file) => {
                let mut data = [0; CHUNK_SIZE];
                let bytes_read = file.read(&mut data).unwrap();
                if bytes_read > 0 {
                    let chunk = Message::Chunk(Vec::from(&data[0..bytes_read]));
                    let output_data = bincode::serialize(&chunk).unwrap();
                    handler.network().send(self.destination, &output_data);
                    self.file_bytes_sent += bytes_read;

                    handler
                        .signals()
                        .send_with_timer(Signal::SendChunk, Duration::from_micros(10));
                    Some(bytes_read)
                } else {
                    println!("File sent!");
                    None
                }
            }
            None => None,
        }
    }
}
