use crate::{
    behaviour::DhtBehaviour,
    blockchain::{block::Block, transaction::Transaction},
    gossip,
    state::State,
};
use libp2p::{
    Swarm,
    kad::{Quorum, Record, RecordKey},
};
use libp2p_gossipsub::IdentTopic;
use serde_json::to_vec;
use std::{error::Error, num::NonZeroUsize};

pub struct Runtime {
    pub swarm: Swarm<DhtBehaviour>,
    pub state: State,
}

impl Runtime {
    pub fn new(swarm: Swarm<DhtBehaviour>, state: State) -> Self {
        Self { swarm, state }
    }

    /// Function validates and appends to chain a block received over gossip protocol.
    /// If the block is valid it is gossiped along
    pub fn accept_block(&mut self, block: Block) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.state.blockchain.accept_block(block.clone())?;
        self.swarm
            .behaviour_mut()
            .gossip
            .publish(IdentTopic::new(gossip::topic::BLOCKS), to_vec(&block)?)?;
        Ok(())
    }

    /// Validates and adds a transaction to the mempool, then gossips it to peers.
    pub fn submit_transaction(
        &mut self,
        transaction: Transaction,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        transaction.verify()?;

        if !self
            .state
            .blockchain
            .transaction_mempool
            .contains(&transaction)
        {
            self.state
                .blockchain
                .transaction_mempool
                .add_transaction(transaction.clone())?;

            self.swarm.behaviour_mut().gossip.publish(
                IdentTopic::new(gossip::topic::TRANSACTIONS),
                to_vec(&transaction)?,
            )?;
        }

        Ok(())
    }

    pub fn load_from_local(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        for rec in &self.state.local.value_records {
            let quorum = match NonZeroUsize::new(rec.quorum) {
                Some(q) => q,
                None => return Err("Stored quorum must be greater than zero".into()),
            };

            let record = Record {
                key: RecordKey::new(&rec.key),
                value: rec.value.clone(),
                publisher: None,
                expires: None,
            };

            self.swarm
                .behaviour_mut()
                .kad
                .put_record(record, Quorum::N(quorum))?;
        }

        for rec in &self.state.local.provider_records {
            self.swarm
                .behaviour_mut()
                .kad
                .start_providing(RecordKey::new(&rec.key))?;
        }

        let _ = self.swarm.behaviour_mut().kad.bootstrap();

        Ok(())
    }
}
