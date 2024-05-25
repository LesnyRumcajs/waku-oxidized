use waku_oxidized::{WakuLightNode, WakuLightNodeConfig};

fn main() -> anyhow::Result<()> {
    let config = WakuLightNodeConfig::new(None, Vec::new());
    let node = WakuLightNode::new_with_config(config)?;
    for peer in node.swarm.connected_peers() {
        println!("peer {}", peer);
    }
    std::thread::sleep(std::time::Duration::from_secs(5));
    Ok(())
}
