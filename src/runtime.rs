use crate::{
    behaviour::{DhtBehaviour, Request},
    blockchain::block::Block,
    state::State,
    topic::BLOCKS,
};
use libp2p::{PeerId, Swarm};
use libp2p_gossipsub::IdentTopic;
use serde_json::to_vec;
use std::{error::Error, sync::Arc};
use tokio::sync::RwLock;

pub struct Runtime {
    pub swarm: Swarm<DhtBehaviour>,
    pub state: Arc<RwLock<State>>,
}

impl Runtime {
    pub fn new(swarm: Swarm<DhtBehaviour>, state: State) -> Self {
        Self {
            swarm,
            state: Arc::new(RwLock::new(state)),
        }
    }

    pub async fn accept_block_from_gossip(
        &mut self,
        block: Block,
        peer: PeerId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.accept_block(block, peer, true, true).await
    }

    pub async fn accept_block_from_r_r(
        &mut self,
        block: Block,
        peer: PeerId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.accept_block(block, peer, false, false).await
    }

    /// Function validates and appends to chain a block received over gossip protocol.
    /// If the block is valid it gossips the block.
    async fn accept_block(
        &mut self,
        block: Block,
        peer: PeerId,
        rebroadcast: bool,
        request_missing: bool,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let result = self
            .state
            .write()
            .await
            .blockchain
            .accept_block(block.clone());

        match result {
            Ok(_) => {
                tracing::info!("Accepted block: {:?}", block);

                if rebroadcast {
                    self.swarm
                        .behaviour_mut()
                        .gossip
                        .publish(IdentTopic::new(BLOCKS), to_vec(&block)?)?;
                }
            }

            // Spaguetti Logic in error handling.
            // Need to implement a propper error module
            Err(e) => {
                let msg = e.to_string();

                if msg == "Already known block." {
                    return Ok(());
                }

                if msg == "The block proposed does not point to a block in the chain." {
                    self.state
                        .write()
                        .await
                        .received_blocks
                        .insert(block.hash.clone(), block.clone());

                    if request_missing {
                        self.swarm
                            .behaviour_mut()
                            .request_response
                            .send_request(&peer, Request::LongestChainHashes);
                    }
                }
            }
        }

        Ok(())
    }

    /// Adjusts a peer's application score by a given delta and
    /// sets syncs application score in gossip sub
    pub async fn adjust_score(
        &mut self,
        peer_id: &PeerId,
        delta: f64,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut state = self.state.write().await;
        let entry = state.peers.entry(peer_id.clone()).or_default();

        entry.application_score += delta;
        let score = entry.application_score;
        self.swarm
            .behaviour_mut()
            .gossip
            .set_application_score(peer_id, score);

        Ok(())
    }

    /// Replays persistent blacklist into gossipsub and bootstraps Kademlia.
    pub async fn load_from_local(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let blacklisted: Vec<PeerId> = self
            .state
            .read()
            .await
            .peers
            .iter()
            .filter_map(|(id, info)| if info.blacklisted { Some(*id) } else { None })
            .collect();

        for peer_id in blacklisted {
            self.swarm.behaviour_mut().gossip.blacklist_peer(&peer_id);
        }

        let _ = self.swarm.behaviour_mut().kad.bootstrap();
        Ok(())
    }
}
