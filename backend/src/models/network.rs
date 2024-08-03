use super::message::NetMessage;
use message_io::network::Endpoint;

pub trait Network {
    fn send_to_target(&self, target: Endpoint, message: &NetMessage);
    fn send_to_all(&self, message: &NetMessage);
}
