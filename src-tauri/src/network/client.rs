use std::{collections::HashSet, error::Error, path::Path, time::{Duration, Instant}};

use futures::{
    channel::{mpsc, oneshot},
    prelude::*,
};
use iroh::endpoint::presets;
use iroh::{Endpoint, SecretKey};
use iroh_blobs::{BlobsProtocol, store::fs::FsStore};

use crate::network::file_response::FileResponse;
use crate::network::{FileData, command::Command};

#[derive(Clone)]
pub struct Client {
    sender: mpsc::Sender<Command>,
}

impl Client {
    pub(crate) fn new(sender: mpsc::Sender<Command>) -> Self {
        Client { sender }
    }

    /// Listen for incoming connections on the given address.
    pub(crate) async fn start_listening(&mut self) -> Result<(), std::error::Error> {
        let secret_key = SecretKey::generate();
        let mut builder = Endpoint::builder(presets::N0)
            .alpns(vec![iroh_blobs::protocol::ALPN.to_vec()])
            .secret_key(secret_key)
            .relay_mode(iroh::RelayMode::Default);

        let t0 = Instant::now();
        let endpoint = builder.bind().await?;

        // TODO: change this to load file path from database storage. Or at least what we're providing with.
        let file_path = Path::new("./../../blender_rs/examples/assets/test.blend");
        let store  = FsStore::load(file_path).await?;
        let blobs = BlobsProtocol::new(
            &store,
        )


        Ok(())
    }

    /// Advertise the local node as the provider of the given file on the DHT.
    pub(crate) async fn start_providing(&mut self, file_name: String) {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::StartProviding { file_name, sender })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.");
    }

    /// Find the providers for the given file on the DHT.
    #[allow(dead_code)]
    pub(crate) async fn get_providers(&mut self, file_name: String) -> HashSet<PeerId> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::GetProviders { file_name, sender })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    /// Request the content of the given file from the given peer.
    #[allow(dead_code)]
    pub(crate) async fn request_file(
        &mut self,
        peer: PeerId,
        file_name: String,
    ) -> Result<Vec<u8>, Box<dyn Error + Send>> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::RequestFile {
                file_name,
                peer,
                sender,
            })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not be dropped.")
    }

    /// Respond with the provided file content to the given request.
    pub(crate) async fn respond_file(
        &mut self,
        file: FileData,
        channel: ResponseChannel<FileResponse>,
    ) {
        self.sender
            .send(Command::RespondFile { file, channel })
            .await
            .expect("Command receiver not to be dropped.");
    }
}
