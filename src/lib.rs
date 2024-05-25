use libp2p::{
    identity::Keypair, noise, request_response, tcp, yamux, Multiaddr, PeerId, StreamProtocol,
    Swarm,
};
use log::{error, info};
pub mod peer_exchange;

pub struct WakuLightNode {
    pub swarm: Swarm<request_response::Behaviour<peer_exchange::Codec>>,
}

pub struct WakuLightNodeConfig {
    pub peers: Vec<Multiaddr>,
    pub keypair: Keypair,
}

impl WakuLightNodeConfig {
    pub fn new(keypair: Option<Keypair>, peers: Vec<Multiaddr>) -> Self {
        Self {
            keypair: keypair.unwrap_or(Keypair::generate_ed25519()),
            peers,
        }
    }
}

impl WakuLightNode {
    pub fn new_with_config(config: WakuLightNodeConfig) -> Result<Self, Error> {
        let local_peer_id = PeerId::from(config.keypair.public());
        info!("Libp2p local peer id: {:?}", local_peer_id);

        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(config.keypair)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|_key| {
                request_response::Behaviour::new(
                    [(
                        // TODO what should the protocol name be
                        StreamProtocol::new("/SOME_NAME"),
                        request_response::ProtocolSupport::Full,
                    )],
                    request_response::Config::default(),
                )
            })
            .unwrap()
            .build();

        for peer in config.peers {
            swarm.dial(peer)?;
        }
        Ok(Self { swarm })
    }

    pub fn request_peers(&mut self, peer: &PeerId) {
        self.swarm.behaviour_mut().send_request(
            peer,
            peer_exchange::messages::PeerExchangeQuery { num_peers: 5 },
        );
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Multiaddr: {0}")]
    Multiaddr(#[from] libp2p::multiaddr::Error),
    #[error("Transport: {0}")]
    Transport(#[from] libp2p::TransportError<std::io::Error>),
    #[error("Dial: {0}")]
    Dial(#[from] libp2p::swarm::DialError),
    #[error("Noise: {0}")]
    Noise(#[from] libp2p::noise::Error),
}
