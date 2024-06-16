use message_io::network::{NetEvent, Transport};
use message_io::node::{self};

use crate::models::render_node::RenderNode;

pub fn multicast() {
    // multicast feature
    let (handler, listener) = node::split::<()>();
    let multicast_addr = "239.255.0.1:15000";
    let (endpoint, _) = handler
        .network()
        .connect(Transport::Udp, multicast_addr)
        .unwrap();

    listener.for_each(move |event| match event.network() {
        NetEvent::Connected(_, _always_true_for_udp) => {
            println!("Notifying on the network");
            handler.network().send(
                endpoint,
                "TODO: Insert blender farm computer name here".as_bytes(),
            );
            // Open UDP channel to receive any incoming file from the host.
            handler
                .network()
                .listen(Transport::Udp, multicast_addr)
                .unwrap();
        }
        // TODO: research why this is implemented but unreachable?
        NetEvent::Accepted(_, _) => unreachable!(),
        NetEvent::Message(endpoint, data) => {
            let message = String::from_utf8_lossy(data);
            // here we will parse data from json into struct type

            println!("{} has connected to the network!", message);
            let render_node = RenderNode::new(message.to_string(), endpoint.addr());
            dbg!(render_node);
            // let message: SenderMsg = bincode::deserialize(data).unwrap();
        }
        NetEvent::Disconnected(endpoint) => {
            println!("{} has left the network", endpoint.addr().ip());

            // if transfers.contains_key(&endpoint) {
            //     transfers.remove(&endpoint);
            // }
        }
    })
}
