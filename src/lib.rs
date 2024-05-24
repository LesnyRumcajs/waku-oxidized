use libp2p::{identity::Keypair, PeerId};
use log::{error, info};

pub struct WakuLightNode {}

impl WakuLightNode {
    pub fn new(keypair: Option<Keypair>, _pubsub_topic: Option<String>) -> Result<Self, Error> {
        let local_key = keypair.unwrap_or(Keypair::generate_ed25519());
        let local_peer_id = PeerId::from(local_key.public());
        info!("Libp2p local peer id: {:?}", local_peer_id);

        Ok(Self {})
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Multiaddr: {0}")]
    Multiaddr(#[from] libp2p::multiaddr::Error),
    #[error("Transport: {0}")]
    Transport(#[from] libp2p::TransportError<std::io::Error>),
}
