use waku_oxidized::{WakuLightNode, WakuLightNodeConfig};

fn main() -> anyhow::Result<()> {
    let config = WakuLightNodeConfig::new(None, Vec::new());
    let _swarm = WakuLightNode::new_with_config(config)?;
    Ok(())
}
