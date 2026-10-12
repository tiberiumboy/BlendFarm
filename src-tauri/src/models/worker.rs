use super::computer_spec::ComputerSpec;
use iroh::EndpointId;

// Treat this struct as peers found on the network
#[derive(Debug)]
pub struct Worker {
    id: EndpointId,
    pub spec: ComputerSpec,
    // internally, we should at least documented the logs and entry.
    // logs: Vec<ServerEvent>,
}

impl Worker {
    pub fn new(id: EndpointId, spec: ComputerSpec) -> Self {
        Self { id, spec }
    }

    // Consider implementing SSE (Stream Server Event) stream to monitor activity.
    // https://www.youtube.com/watch?v=5rtUveDkERc

    // This worker class should implements several things:
    // a way to stream data to the caller and
    // a way to receive said stream and process events
    // async fn serve_sse(&self) -> Result<Receiver<Status>, Box<dyn Error>> {
    //     let rx = todo!("Supply Blender's events")subscribe().await?; // get the hooks of reciever signal somewhere? Blender?
    //     let stream = rx
    //         .into_stream()
    //         .map(|change| Status::default().json_data(change));
    //     Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
    // }
}

#[cfg(test)]
mod tests {
    // use iroh::Endpoint;

    // use super::*;

    // #[test]
    // fn ensure_new_worker_succeed() {
    //     let spec = ComputerSpec::new();
    //     let id = Endpoint::
    //     let worker = Worker::new(spec.clone());
    //     assert_eq!(worker.spec, spec);
    // }
}
