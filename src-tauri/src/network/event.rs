use libp2p_request_response::ResponseChannel;

use crate::network::file_response::FileResponse;

#[derive(Debug)]
pub(crate) enum Event {
    InboundRequest {
        request: String,
        channel: ResponseChannel<FileResponse>,
    },
}
