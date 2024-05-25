use libp2p::{
    identity::Keypair, noise, request_response, swarm::NetworkBehaviour, tcp, yamux, Multiaddr,
    PeerId, StreamProtocol, Swarm,
};
use log::{error, info};
use peer_exchange::messages;

pub mod peer_exchange;

use std::time::Duration;

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

pub struct WakuLightNode {
    pub swarm: Swarm<WakuLightNodeBehaviour>,
}

impl WakuLightNode {
    pub fn new_with_config(config: WakuLightNodeConfig) -> Result<Self, Error> {
        let local_peer_id = PeerId::from(config.keypair.public());
        info!("Libp2p local peer id: {:?}", local_peer_id);

        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(config.keypair)
            .with_tokio()
            .with_tcp(
                tcp::Config::default().nodelay(true),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_dns()?
            .with_behaviour(|_key| WakuLightNodeBehaviour::new())
            .unwrap()
            .with_swarm_config(|config| {
                config
                    .with_notify_handler_buffer_size(
                        std::num::NonZeroUsize::new(20).expect("Not zero"),
                    )
                    .with_per_connection_event_buffer_size(64)
                    .with_idle_connection_timeout(Duration::from_secs(60 * 10))
            })
            .build();

        for peer in config.peers {
            swarm.dial(peer)?;
        }
        Ok(Self { swarm })
    }

    pub fn request_peers(&mut self, peer: &PeerId) {
        self.swarm.behaviour_mut().peer_exchange.send_request(
            peer,
            peer_exchange::messages::PeerExchangeQuery { num_peers: 5 },
        );
    }
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "WakuLightNodeEvent")]
pub struct WakuLightNodeBehaviour {
    peer_exchange: request_response::Behaviour<peer_exchange::Codec>,
}

impl WakuLightNodeBehaviour {
    fn new() -> Self {
        Self {
            peer_exchange: request_response::Behaviour::new(
                [(
                    StreamProtocol::new("/vac/waku/peer-exchange/2.0.0-alpha1"),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            ),
        }
    }
}

#[derive(Debug)]
pub enum WakuLightNodeEvent {
    PeerExchange(
        request_response::Event<messages::PeerExchangeQuery, messages::PeerExchangeResponse>,
    ),
}

impl From<request_response::Event<messages::PeerExchangeQuery, messages::PeerExchangeResponse>>
    for WakuLightNodeEvent
{
    fn from(
        event: request_response::Event<messages::PeerExchangeQuery, messages::PeerExchangeResponse>,
    ) -> Self {
        Self::PeerExchange(event)
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
    #[error("Io: {0}")]
    Io(#[from] std::io::Error),
}
