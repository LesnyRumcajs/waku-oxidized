use filter::messages::filter_subscribe_request::FilterSubscribeType;
use libp2p::{
    identity::Keypair, noise, request_response, swarm::NetworkBehaviour, tcp, yamux, Multiaddr,
    PeerId, StreamProtocol, Swarm,
};
use log::{error, info};

mod filter;
mod light_push;
mod metadata;
mod peer_exchange;

use std::{
    num::TryFromIntError,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const DEFAULT_PUBSUB_TOPIC: &str = "/waku/2/default-waku/proto";

pub struct WakuLightNodeConfig {
    /// Initial nodes to connect to
    pub peers: Vec<Multiaddr>,
    /// A libp2p identity keypair
    pub keypair: Keypair,
}

impl WakuLightNodeConfig {
    /// Create config, generating a keypair unless one is given
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
            .unwrap() // Infalliable
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

    /// Send a Waku message
    pub fn send_message(
        &mut self,
        peer: &PeerId,
        content_topic: String,
        payload: Vec<u8>,
    ) -> Result<(), Error> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs()
            .try_into()?;

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
                        timestamp: Some(timestamp),
                        ..Default::default()
                    }),
                }),
            },
        );
        Ok(())
    }

    /// Subscribe to topic(s) using the filter protocol
    pub fn filter_subscribe(&mut self, peer: &PeerId, content_topics: Vec<String>) {
        self.swarm.behaviour_mut().filter.send_request(
            peer,
            filter::FilterSubscribeRequest {
                pubsub_topic: Some(DEFAULT_PUBSUB_TOPIC.to_string()),
                content_topics,
                request_id: "0".to_string(),
                filter_subscribe_type: FilterSubscribeType::Subscribe as i32,
            },
        );
    }

    /// Unsubscribe from topic(s) using the filter protocol
    pub fn filter_unsubscribe(&mut self, peer: &PeerId, content_topics: Vec<String>) {
        self.swarm.behaviour_mut().filter.send_request(
            peer,
            filter::messages::FilterSubscribeRequest {
                pubsub_topic: Some(DEFAULT_PUBSUB_TOPIC.to_string()),
                content_topics,
                request_id: "0".to_string(),
                filter_subscribe_type: FilterSubscribeType::Unsubscribe as i32,
            },
        );
    }
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "WakuLightNodeEvent")]
pub struct WakuLightNodeBehaviour {
    peer_exchange: request_response::Behaviour<peer_exchange::Codec>,
    metadata: request_response::Behaviour<metadata::Codec>,
    light_push: request_response::Behaviour<light_push::Codec>,
    filter: request_response::Behaviour<filter::Codec>,
}

impl WakuLightNodeBehaviour {
    fn new() -> Self {
        Self {
            peer_exchange: request_response::Behaviour::new(
                [(
                    StreamProtocol::new(peer_exchange::PROTOCOL_NAME),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            ),
            metadata: request_response::Behaviour::new(
                [(
                    StreamProtocol::new(metadata::PROTOCOL_NAME),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            ),
            light_push: request_response::Behaviour::new(
                [(
                    StreamProtocol::new(light_push::PROTOCOL_NAME),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            ),
            filter: request_response::Behaviour::new(
                [(
                    StreamProtocol::new(filter::PROTOCOL_NAME),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            ),
        }
    }
}

/// An event from one of the Waku light node protocols
#[derive(Debug)]
pub enum WakuLightNodeEvent {
    PeerExchange(
        request_response::Event<
            peer_exchange::messages::PeerExchangeRpc,
            peer_exchange::messages::PeerExchangeRpc,
        >,
    ),
    Metadata(
        request_response::Event<
            metadata::messages::WakuMetadataRequest,
            metadata::messages::WakuMetadataResponse,
        >,
    ),
    LightPush(
        request_response::Event<light_push::messages::PushRpc, light_push::messages::PushRpc>,
    ),
    Filter(
        request_response::Event<
            filter::messages::FilterSubscribeRequest,
            filter::messages::FilterSubscribeResponse,
        >,
    ),
}

impl
    From<
        request_response::Event<
            peer_exchange::messages::PeerExchangeRpc,
            peer_exchange::messages::PeerExchangeRpc,
        >,
    > for WakuLightNodeEvent
{
    fn from(
        event: request_response::Event<
            peer_exchange::messages::PeerExchangeRpc,
            peer_exchange::messages::PeerExchangeRpc,
        >,
    ) -> Self {
        Self::PeerExchange(event)
    }
}

impl
    From<
        request_response::Event<
            filter::messages::FilterSubscribeRequest,
            filter::messages::FilterSubscribeResponse,
        >,
    > for WakuLightNodeEvent
{
    fn from(
        event: request_response::Event<
            filter::messages::FilterSubscribeRequest,
            filter::messages::FilterSubscribeResponse,
        >,
    ) -> Self {
        Self::Filter(event)
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

/// Error when setting up or running a light node
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
