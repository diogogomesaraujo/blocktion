pub mod behaviour;
pub mod boot;
pub mod config;
pub mod node;
pub mod rpc;

//topic for gossip behaviour
pub struct Topic;
impl Topic {
    pub const TRANSACTIONS: &str = "transactions";
    pub const BLOCKS: &str = "blocks";
    pub const OVERLAY_META: &str = "overlay-meta";
}
