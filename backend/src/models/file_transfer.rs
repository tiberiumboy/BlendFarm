use super::message::NetMessage;
use message_io::{network::Endpoint, node::NodeHandler};
use std::{fs::File, io::Read, path::PathBuf};

const CHUNK_SIZE: usize = 65536;

#[derive(Debug)]
pub struct FileTransfer {
    // pub file_path: PathBuf,
    pub file: File,
    // pub size: usize,
    pub destination: Endpoint,
    pub file_bytes_sent: usize,
}

impl FileTransfer {
    pub fn new(file_path: PathBuf, destination: Endpoint) -> Self {
        let file = File::open(&file_path).unwrap();
        // let size = fs::metadata(&file_path).unwrap().len() as usize;

        FileTransfer {
            // file_path,
            file,
            // size,
            destination,
            file_bytes_sent: 0,
        }
    }
    /*

    pub fn transfer(&mut self, handler: &NodeHandler<NetMessage>) -> Option<usize> {
        // Does this mean that the amount of data I can hold in memory is 65536 bytes?
        let mut data = [0; CHUNK_SIZE];
        let bytes_read = self.file.read(&mut data).unwrap();
        if bytes_read > 0 {
            let chunk = NetMessage::Chunk(Vec::from(&data[0..bytes_read]));
            let output_data = bincode::serialize(&chunk).unwrap();
            handler.network().send(self.destination, &output_data);
            self.file_bytes_sent += bytes_read;

            // handler
            //     .signals()
            //     // is it necessary to have a timer here?
            //     .send_with_timer(Signal::SendChunk, Duration::from_micros(10));
            Some(bytes_read)
        } else {
            println!("File sent!");
            None
        }
    }

     */
}
