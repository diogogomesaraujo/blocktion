#[cfg(test)]
pub mod test {
    use crate::blockchain::block::Block;
    use crate::blockchain::hash::{encode_hash, hash};
    use crate::blockchain::transaction::TransactionPool;
    use crate::blockchain::{Blockchain, WorldState};
    use crate::blockchain::{
        ed25519::public_key_to_string,
        transaction::{Data, Transaction},
    };
    use blake2::{Blake2b512, Digest};
    use ed25519_dalek_blake2b::Keypair;
    use rand::rngs::OsRng;
    use std::error::Error;

    use super::test_utils::{generate_keypair, signed_create_account_tx};

    #[test]
    fn test_hash() {
        let to_hash = "I am not in danger, Skyler. I am the danger.";
        let hashed = hash(Blake2b512::new(), to_hash);

        assert_eq!(
            encode_hash(&hashed),
            "3a141d45dea6b8af5bab5f942d88f3c0d48edcda84fac341d821d13d65896e2a7d5a8ec921da654301e72db33631fd94963e064056172f4d970a77625aa7ed93"
        );
    }

    pub fn test_mempool() -> Result<(), Box<dyn Error + Send + Sync>> {
        let k1 = generate_keypair();
        let k2 = generate_keypair();

        let t1 = Transaction::sign(
            Data::CreateUserAccount {
                public_key: "skylar".to_string(),
            },
            &public_key_to_string(&k1.public),
            0,
            &k1,
        )?;

        let t2 = Transaction::sign(
            Data::CreateUserAccount {
                public_key: "walter".to_string(),
            },
            &public_key_to_string(&k2.public),
            1,
            &k2,
        )?;

        let mut pool = TransactionPool::new();
        pool.add_transaction(t1.clone())?;
        pool.add_transaction(t2.clone())?;

        assert_eq!(
            pool.flush().into_iter().collect::<Vec<Transaction>>(),
            vec![t1, t2]
        );

        Ok(())
    }

    /// Tests that a transaction added to the pool can be found via contains()
    #[test]
    fn test_pool_added_transaction_is_contained() -> Result<(), Box<dyn Error + Send + Sync>> {
        let k = generate_keypair();
        let t = signed_create_account_tx(&k, 0)?;
        let mut pool = TransactionPool::new();
        pool.add_transaction(t.clone())?;
        assert!(pool.contains(&t));
        Ok(())
    }

    /// Tests that flush() drains the pool and returns all transactions sorted by timestamp
    #[test]
    fn test_pool_flush_returns_transactions_and_empties_pool()
    -> Result<(), Box<dyn Error + Send + Sync>> {
        let k1 = generate_keypair();
        let k2 = generate_keypair();
        let t1 = signed_create_account_tx(&k1, 0)?;
        let t2 = signed_create_account_tx(&k2, 0)?;
        let mut pool = TransactionPool::new();
        pool.add_transaction(t1.clone())?;
        pool.add_transaction(t2.clone())?;
        let queue = pool.flush();
        assert_eq!(queue.len(), 2);
        assert_eq!(pool.len(), 0);
        // verify sorted by timestamp
        assert!(queue[0].created_at <= queue[1].created_at);
        Ok(())
    }

    /// Tests that remove() correctly deletes a transaction from the pool by timestamp
    #[test]
    fn test_pool_remove_deletes_transaction() -> Result<(), Box<dyn Error + Send + Sync>> {
        let k = generate_keypair();
        let t = signed_create_account_tx(&k, 0)?;
        let mut pool = TransactionPool::new();
        pool.add_transaction(t.clone())?;
        pool.remove(t.id.clone());
        assert_eq!(pool.len(), 0);
        assert!(!pool.contains(&t));
        Ok(())
    }

    /* Transaction */

    /// Tests that a correctly signed transaction passes verification
    #[test]
    fn test_transaction_valid_signature_verifies() -> Result<(), Box<dyn Error + Send + Sync>> {
        let k = generate_keypair();
        let t = signed_create_account_tx(&k, 0)?;
        assert!(t.verify().is_ok());
        Ok(())
    }

    /// Tests that tampering with the nonce after signing invalidates the transaction
    #[test]
    fn test_transaction_tampered_nonce_fails_verification()
    -> Result<(), Box<dyn Error + Send + Sync>> {
        let k = generate_keypair();
        let mut t = signed_create_account_tx(&k, 0)?;
        t.nonce = 999;
        assert!(t.verify().is_err());
        Ok(())
    }

    /// Tests that tampering with the record after signing invalidates the transaction
    #[test]
    fn test_transaction_tampered_record_fails_verification()
    -> Result<(), Box<dyn Error + Send + Sync>> {
        let k = generate_keypair();
        let mut t = signed_create_account_tx(&k, 0)?;
        t.record = Data::CreateUserAccount {
            public_key: "imposter".to_string(),
        };
        assert!(t.verify().is_err());
        Ok(())
    }

    /// Tests that a transaction signed by one keypair cannot be verified with another keypair's public key
    #[test]
    fn test_transaction_wrong_keypair_fails_verification()
    -> Result<(), Box<dyn Error + Send + Sync>> {
        let k1 = generate_keypair();
        let k2 = generate_keypair();
        let mut t = signed_create_account_tx(&k1, 0)?;
        t.from = public_key_to_string(&k2.public); // swap sender to different key
        assert!(t.verify().is_err());
        Ok(())
    }

    /// Tests that two different transactions produce different IDs
    #[test]
    fn test_transaction_unique_ids() -> Result<(), Box<dyn Error + Send + Sync>> {
        let k1 = generate_keypair();
        let k2 = generate_keypair();
        let t1 = signed_create_account_tx(&k1, 0)?;
        let t2 = signed_create_account_tx(&k2, 0)?;
        assert_ne!(t1.id, t2.id);
        Ok(())
    }

    /// Tests that a freshly mined block passes its own verification
    #[test]
    fn test_block_mined_block_is_valid() -> Result<(), Box<dyn Error + Send + Sync>> {
        let k = generate_keypair();
        let pk = public_key_to_string(&k.public);
        let t = signed_create_account_tx(&k, 0)?;
        let block = Block::new(pk, None, vec![t], u32::MAX)?;
        assert!(block.verify()?);
        Ok(())
    }

    /// Tests that tampering with the nonce after mining invalidates the block
    #[test]
    fn test_block_tampered_nonce_fails_verification() -> Result<(), Box<dyn Error + Send + Sync>> {
        let k = generate_keypair();
        let pk = public_key_to_string(&k.public);
        let t = signed_create_account_tx(&k, 0)?;
        let mut block = Block::new(pk, None, vec![t], u32::MAX)?;
        block.nonce = block.nonce.wrapping_add(1);
        assert!(!block.verify()?);
        Ok(())
    }

    /// Tests that replacing the stored hash with a different value fails verification
    #[test]
    fn test_block_tampered_hash_fails_verification() -> Result<(), Box<dyn Error + Send + Sync>> {
        let k = generate_keypair();
        let pk = public_key_to_string(&k.public);
        let t = signed_create_account_tx(&k, 0)?;
        let mut block = Block::new(pk, None, vec![t], u32::MAX)?;
        block.hash = "00".repeat(64);
        assert!(!block.verify()?);
        Ok(())
    }

    /// Tests that tampering with a transaction inside a mined block fails verification
    #[test]
    fn test_block_tampered_transaction_fails_verification()
    -> Result<(), Box<dyn Error + Send + Sync>> {
        let k = generate_keypair();
        let pk = public_key_to_string(&k.public);
        let t = signed_create_account_tx(&k, 0)?;
        let mut block = Block::new(pk, None, vec![t], u32::MAX)?;
        block.transactions[0].nonce = 999;
        assert!(!block.verify()?);
        Ok(())
    }

    /* WorldState */

    /// Tests that a created account can be retrieved and a non-existent one returns None
    #[test]
    fn test_worldstate_account_creation_and_lookup() -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut chain = Blockchain::new(u32::MAX)?;
        chain.create_account("rick_pk")?;
        assert!(chain.get_account_by_id("rick_pk").is_some());
        assert!(chain.get_account_by_id("morty_pk").is_none());
        Ok(())
    }

    /// Tests that attempting to create an account with an already existing public key fails
    #[test]
    fn test_worldstate_duplicate_account_is_rejected() -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut chain = Blockchain::new(u32::MAX)?;
        chain.create_account("rick_pk")?;
        assert!(chain.create_account("rick_pk").is_err());
        assert_eq!(chain.accounts.len(), 1);
        Ok(())
    }

    /* accept_block() */

    /// Tests that accept_block() rejects a block whose hash does not satisfy PoW
    #[test]
    fn test_accept_block_rejects_invalid_pow() -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut chain = Blockchain::new(u32::MAX)?;
        let shady_block = Block {
            previous_hash: "0".to_string(),
            transactions: vec![],
            merkle_root: "".to_string(),
            hash: "not_a_valid_pow_hash".to_string(),
            nonce: 0,
            timestamp: 1,
            miner: "".to_string(),
        };
        assert!(chain.accept_block(shady_block).is_err());
        Ok(())
    }

    /// Tests that accept_block() rejects a block that does not point to the current chain tip
    #[test]
    fn test_accept_block_rejects_wrong_previous_hash() -> Result<(), Box<dyn Error + Send + Sync>> {
        let k = generate_keypair();
        let pk = public_key_to_string(&k.public);
        let t = signed_create_account_tx(&k, 0)?;
        let mut block = Block::new(pk.clone(), None, vec![t.clone()], u32::MAX)?;
        block.previous_hash = "wrong".to_string();

        let mut chain = Blockchain::new(u32::MAX)?;
        chain.transaction_pool.add_transaction(t)?;

        assert!(chain.accept_block(block).is_err());
        Ok(())
    }

    /// Tests that accept_block() rejects a block whose transactions are not in the mempool
    #[test]
    fn test_accept_block_rejects_transactions_not_in_mempool()
    -> Result<(), Box<dyn Error + Send + Sync>> {
        let k = generate_keypair();
        let pk = public_key_to_string(&k.public);
        let t = signed_create_account_tx(&k, 0)?;
        let block = Block::new(pk.clone(), None, vec![t], u32::MAX)?;

        let mut chain = Blockchain::new(u32::MAX)?;
        chain.create_account(&pk)?;
        // intentionally not adding t to the mempool

        assert!(chain.accept_block(block).is_err());
        Ok(())
    }

    /// Tests that accept_block() rejects a block proposed by an unknown miner
    #[test]
    fn test_accept_block_rejects_unknown_miner() -> Result<(), Box<dyn Error + Send + Sync>> {
        let k = generate_keypair();
        let pk = public_key_to_string(&k.public);
        let t = signed_create_account_tx(&k, 0)?;
        let block = Block::new(pk.clone(), None, vec![t.clone()], u32::MAX)?;

        let mut chain = Blockchain::new(u32::MAX)?;
        chain.transaction_pool.add_transaction(t)?;
        // intentionally not registering pk as an account

        assert!(chain.accept_block(block).is_err());
        Ok(())
    }

    /* Blockchain */

    /// Tests that an empty blockchain passes verification
    #[test]
    fn test_blockchain_empty_chain_is_valid() -> Result<(), Box<dyn Error + Send + Sync>> {
        let chain = Blockchain::new(u32::MAX)?;
        assert!(chain.verify()?);
        Ok(())
    }

    /// Tests that propose_block() mines, commits the block, and the chain remains valid
    #[test]
    fn test_blockchain_propose_block_grows_chain() -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut chain = Blockchain::new(u32::MAX)?;
        let k = generate_keypair();
        let pk = public_key_to_string(&k.public);
        let t = signed_create_account_tx(&k, 0)?;
        chain.transaction_pool.add_transaction(t)?;
        chain.propose_block(pk.clone())?;
        assert_eq!(chain.blocks.len(), 1);
        assert!(chain.verify()?);
        Ok(())
    }

    /// Tests that propose_block() fails when the mempool is empty
    #[test]
    fn test_blockchain_propose_block_fails_with_empty_mempool()
    -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut chain = Blockchain::new(u32::MAX)?;
        let k = generate_keypair();
        let pk = public_key_to_string(&k.public);
        assert!(chain.propose_block(pk).is_err());
        Ok(())
    }

    /// Tests that propose_block() correctly executes transactions, creating the account on-chain
    #[test]
    fn test_blockchain_propose_block_executes_transactions()
    -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut chain = Blockchain::new(u32::MAX)?;
        let k = generate_keypair();
        let pk = public_key_to_string(&k.public);
        let t = signed_create_account_tx(&k, 0)?;
        chain.transaction_pool.add_transaction(t)?;
        chain.propose_block(pk.clone())?;
        assert!(chain.get_account_by_id(&pk).is_some());
        Ok(())
    }

    /// Tests that a multi-block chain remains valid after sequential propose_block() calls
    #[test]
    fn test_blockchain_multi_block_chain_is_valid() -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut chain = Blockchain::new(u32::MAX)?;
        for _ in 0..3 {
            let k = generate_keypair();
            let pk = public_key_to_string(&k.public);
            let t = signed_create_account_tx(&k, 0)?;
            chain.transaction_pool.add_transaction(t)?;
            chain.propose_block(pk)?;
        }
        assert_eq!(chain.blocks.len(), 3);
        assert!(chain.verify()?);
        Ok(())
    }

    /// Tests that fix() is a no-op on a linear chain with no forks
    #[test]
    fn test_blockchain_fix_is_noop_on_linear_chain() -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut chain = Blockchain::new(u32::MAX)?;
        let k = generate_keypair();
        let pk = public_key_to_string(&k.public);
        let t = signed_create_account_tx(&k, 0)?;
        chain.transaction_pool.add_transaction(t)?;
        chain.propose_block(pk)?;
        let before = chain.blocks.clone();
        chain.fix()?;
        assert_eq!(chain.blocks, before);
        Ok(())
    }

    /// Tests that fix() resolves a fork by keeping the winning branch and returning
    /// the discarded branch's transactions to the mempool
    #[test]
    fn test_blockchain_fix_resolves_fork() -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut blockchain = Blockchain::new(u32::MAX)?;

        let t1 = Transaction::new(
            Data::CreateUserAccount {
                public_key: "skylar".to_string(),
            },
            "walter".to_string(),
            0,
            "i'm the one who knocks",
        )?;

        // b1 and b2 both point to genesis — fork at block 0
        let b1 = Block {
            previous_hash: "0".to_string(),
            transactions: vec![],
            merkle_root: "".to_string(),
            hash: "11".to_string(),
            nonce: 0,
            timestamp: 1,
            miner: "".to_string(),
        };
        let b2 = Block {
            previous_hash: "0".to_string(),
            transactions: vec![],
            merkle_root: "".to_string(),
            hash: "22".to_string(),
            nonce: 0,
            timestamp: 1,
            miner: "".to_string(),
        };
        // b3 extends b2 (the losing branch)
        let b3 = Block {
            previous_hash: "22".to_string(),
            transactions: vec![t1.clone()],
            merkle_root: "".to_string(),
            hash: "33".to_string(),
            nonce: 0,
            timestamp: 1,
            miner: "".to_string(),
        };

        blockchain.blocks.push(b1.clone());
        blockchain.blocks.push(b2);
        blockchain.blocks.push(b3);

        blockchain.fix()?;

        // b1 wins (0x11 < 0x22), b2+b3 are discarded, t1 goes back to mempool
        assert_eq!(blockchain.blocks, vec![b1]);
        assert_eq!(blockchain.transaction_pool.flush(), vec![t1]);
        Ok(())
    }

    #[test]
    fn test_blockchain() -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut blockchain = Blockchain::new(u32::MAX)?;

        for n in 0..1 {
            let keys = Keypair::generate(&mut OsRng);

            let pk = public_key_to_string(&keys.public);
            let t = Transaction::sign(
                Data::CreateUserAccount {
                    public_key: pk.clone(),
                },
                &pk,
                n,
                &keys,
            )?;

            blockchain.transaction_pool.add_transaction(t)?;
            blockchain.propose_block(pk)?;
        }

        // verify blockchain

        assert!(blockchain.verify()?);

        // verify fix function

        let mut fixed_blockchain = blockchain.clone();
        fixed_blockchain.fix()?;

        assert_eq!(blockchain, fixed_blockchain);

        Ok(())
    }
}

pub mod test_utils {
    use crate::blockchain::ed25519::public_key_to_string;
    use crate::blockchain::transaction::{Data, Transaction};
    use ed25519_dalek_blake2b::Keypair;
    use rand::rngs::OsRng;
    use std::error::Error;

    pub fn signed_create_account_tx(
        keys: &Keypair,
        nonce: u32,
    ) -> Result<Transaction, Box<dyn Error + Send + Sync>> {
        let pk = public_key_to_string(&keys.public);
        Transaction::sign(
            Data::CreateUserAccount {
                public_key: pk.clone(),
            },
            &pk,
            nonce,
            keys,
        )
    }

    pub fn generate_keypair() -> Keypair {
        Keypair::generate(&mut OsRng)
    }
}
