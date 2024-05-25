use futures::select;
use libp2p::{futures::StreamExt, swarm::SwarmEvent};
use std::str::FromStr;

use clap::Parser;
use libp2p::Multiaddr;
use waku_oxidized::{WakuLightNode, WakuLightNodeConfig, WakuLightNodeEvent};

#[derive(Parser, Debug, Clone)]
#[clap(version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    peers: Vec<String>,
    topic: String,
    message: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    let config = WakuLightNodeConfig::new(
        None,
        cli.peers
            .iter()
            .map(|peer| Multiaddr::from_str(peer).unwrap())
            .collect(),
    );
    let mut node = WakuLightNode::new_with_config(config)?;

    loop {
        select! {
            swarm_event = node.swarm.next() => match swarm_event {
                Some(SwarmEvent::Behaviour(WakuLightNodeEvent::Metadata(metadata))) => {
                    println!("Got metadata {:?}", metadata);
                    match metadata {
                        libp2p::request_response::Event::Message { peer, message } => {
                            println!("Got message from {:?}: {:?}", peer, message);
                        }
                        libp2p::request_response::Event::OutboundFailure { peer, request_id, error } => {
                            println!("Outbound failure to {:?} for request {:?}: {:?}", peer, request_id, error);
                        }
                        libp2p::request_response::Event::InboundFailure { peer, request_id, error } => {
                            println!("Inbound failure from {:?} for request {:?}: {:?}", peer, request_id, error);
                        }
                        libp2p::request_response::Event::ResponseSent { peer, request_id } => {
                            println!("Response sent to {:?} for request {:?}", peer, request_id);
                        }
                    }
                }
                Some(SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. }) => {
                    println!("Connection estabilished with {peer_id:?} on {endpoint:?}");
                    node.request_peers(&peer_id);
                    node.send_message(&peer_id, cli.topic.clone(), cli.message.clone().into())?;
                }
                None => {
                    println!("Swarm event stream ended");
                    break;
                }
                _ => {
                    println!("Got swarm event {:?}", swarm_event);
                }
            }
        }
    }

    Ok(())
}
