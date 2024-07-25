use super::message::Message;
use message_io::network::Endpoint;

pub trait Network {
    fn send_to_target(&self, target: Endpoint, message: &Message);
    fn send_to_all(&self, message: &Message);
}
