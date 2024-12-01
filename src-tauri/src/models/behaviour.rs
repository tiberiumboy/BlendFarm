use libp2p::{gossipsub, kad, mdns, swarm::NetworkBehaviour};

#[derive(NetworkBehaviour)]
pub struct BlendFarmBehaviour {
    // ping: libp2p::ping::Behaviour,
    pub gossipsub: gossipsub::Behaviour,
    pub mdns: mdns::tokio::Behaviour,
    // Used as a DHT (BitTorrent) for file transfer
    pub kad: kad::Behaviour<kad::store::MemoryStore>, // Ok so I need to figure out how this works? Figure out about TStore trait
}
