use libp2p::{
    identity::Keypair, noise, request_response, swarm::NetworkBehaviour, tcp, yamux, Multiaddr,
    PeerId, StreamProtocol, Swarm,
};
use log::{error, info};
use peer_exchange::messages;

mod light_push;
mod metadata;
mod peer_exchange;

use std::{
    num::TryFromIntError,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const DEFAULT_PUBSUB_TOPIC: &str = "/waku/2/default-waku/proto";

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
            peer_exchange::messages::PeerExchangeRpc {
                query: Some(peer_exchange::messages::PeerExchangeQuery { num_peers: 5 }),
                response: None,
            },
        );
    }

    pub fn send_message(
        &mut self,
        peer: &PeerId,
        content_topic: String,
        payload: Vec<u8>,
    ) -> Result<(), Error> {
        // let timestamp = SystemTime::now()
        //     .duration_since(UNIX_EPOCH)?
        //     .as_secs()
        //     .try_into()?;
        self.swarm.behaviour_mut().light_push.send_request(
            peer,
            light_push::messages::PushRpc {
                request_id: "0".to_owned(),
                response: None,
                request: Some(light_push::messages::PushRequest {
                    pubsub_topic: DEFAULT_PUBSUB_TOPIC.to_string(),
                    message: Some(light_push::message::WakuMessage {
                        content_topic,
                        payload,
                        ephemeral: Some(false),
                        ..Default::default()
                    }),
                }),
            },
        );
        Ok(())
    }
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "WakuLightNodeEvent")]
pub struct WakuLightNodeBehaviour {
    peer_exchange: request_response::Behaviour<peer_exchange::Codec>,
    metadata: request_response::Behaviour<metadata::Codec>,
    light_push: request_response::Behaviour<light_push::Codec>,
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
            metadata: request_response::Behaviour::new(
                [(
                    StreamProtocol::new("/vac/waku/metadata/1.0.0"),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            ),
            light_push: request_response::Behaviour::new(
                [(
                    StreamProtocol::new("/vac/waku/lightpush/2.0.0-beta1"),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            ),
        }
    }
}

#[derive(Debug)]
pub enum WakuLightNodeEvent {
    PeerExchange(request_response::Event<messages::PeerExchangeRpc, messages::PeerExchangeRpc>),
    Metadata(
        request_response::Event<
            metadata::messages::WakuMetadataRequest,
            metadata::messages::WakuMetadataResponse,
        >,
    ),
    LightPush(
        request_response::Event<light_push::messages::PushRpc, light_push::messages::PushRpc>,
    ),
}

impl From<request_response::Event<messages::PeerExchangeRpc, messages::PeerExchangeRpc>>
    for WakuLightNodeEvent
{
    fn from(
        event: request_response::Event<messages::PeerExchangeRpc, messages::PeerExchangeRpc>,
    ) -> Self {
        Self::PeerExchange(event)
    }
}

impl
    From<
        request_response::Event<
            metadata::messages::WakuMetadataRequest,
            metadata::messages::WakuMetadataResponse,
        >,
    > for WakuLightNodeEvent
{
    fn from(
        event: request_response::Event<
            metadata::messages::WakuMetadataRequest,
            metadata::messages::WakuMetadataResponse,
        >,
    ) -> Self {
        Self::Metadata(event)
    }
}

impl From<request_response::Event<light_push::messages::PushRpc, light_push::messages::PushRpc>>
    for WakuLightNodeEvent
{
    fn from(
        event: request_response::Event<
            light_push::messages::PushRpc,
            light_push::messages::PushRpc,
        >,
    ) -> Self {
        Self::LightPush(event)
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
    #[error("System time: {0}")]
    SystemTime(#[from] std::time::SystemTimeError),
    #[error("Int conversion: {0}")]
    IntConversion(#[from] TryFromIntError),
}
