use libp2p::futures::StreamExt;
use std::str::FromStr;

use clap::Parser;
use libp2p::Multiaddr;
use waku_oxidized::{WakuLightNode, WakuLightNodeConfig};

#[derive(Parser, Debug, Clone)]
#[clap(version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    peers: Vec<String>,
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

    while let Some(e) = node.swarm.next().await {
        println!("Got event {:?}", e);
        for peer in node.swarm.connected_peers() {
            println!("Connected to {:?}", peer);
        }
    }

    Ok(())
}
