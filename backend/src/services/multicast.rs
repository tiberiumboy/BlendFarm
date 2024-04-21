use message_io::network::{NetEvent, Transport};
use message_io::node::{self};

#[allow(dead_code)]
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
            handler.network().send(endpoint, "receiver".as_bytes());
            handler
                .network()
                .listen(Transport::Udp, multicast_addr)
                .unwrap();
        }
        NetEvent::Accepted(_, _) => unreachable!(),
        NetEvent::Message(_, data) => {
            let message = String::from_utf8_lossy(data);
            println!("{} has connected to the network!", message);
            // let message: SenderMsg = bincode::deserialize(data).unwrap();
        }
        NetEvent::Disconnected(endpoint) => {
            println!("{} has left the network", endpoint.addr().ip());
            ()
            // if transfers.contains_key(&endpoint) {
            //     transfers.remove(&endpoint);
            // }
        }
    })
}
