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
    let node = WakuLightNode::new_with_config(config)?;
    std::thread::sleep(std::time::Duration::from_secs(5));
    for peer in node.swarm.connected_peers() {
        println!("connected to {:?}", peer);
    }
    Ok(())
}
