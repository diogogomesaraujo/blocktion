use crate::time::now_unix;
use blake2::Blake2b512;
use std::error::Error;

type HashFunction = Blake2b512;

// https://towardsdev.com/the-proof-of-work-pow-mechanism-in-blockchain-6a49196cab75
// https://www.jmeiners.com/tiny-blockchain/
// https://en.bitcoin.it/wiki/Protocol_documentation#Block_Headers

pub mod hash {
    use crate::blockchain::HashFunction;
    use blake2::Digest;

    pub fn hash(mut h: HashFunction, data: &str) -> String {
        h.update(data.as_bytes());
        let bytes = h.finalize().to_vec();
        hex::encode(bytes)
    }
}

pub mod pow {
    use crate::blockchain::{HashFunction, hash};
    use tracing::info;

    const LOG_MINERATION: u32 = 100000;

    pub struct ProofOfWork {
        pub data: String,
        pub difficulty: u32,
    }

    pub fn mine(pow: &ProofOfWork, hasher: HashFunction) -> (String, u32) {
        let prefix = [0..pow.difficulty]
            .iter()
            .fold(String::new(), |acc, _| [acc, String::from("0")].join(""));

        fn mine_rec(
            pow: &ProofOfWork,
            hasher: HashFunction,
            nonce: u32,
            prefix: &str,
        ) -> (String, u32) {
            if nonce % LOG_MINERATION == 0 {
                info!("Still mining. The current nonce value is: {}.", nonce);
            }

            let input = format!("{}:{}", pow.data, nonce);
            let h = hash::hash(hasher.clone(), &input);

            if let Some(_) = h.strip_prefix(&prefix) {
                return (h, nonce);
            }

            mine_rec(pow, hasher, nonce + 1, prefix)
        }

        mine_rec(&pow, hasher, 0, &prefix)
    }
}

pub struct Block {
    pub index: u32,
    pub previous_hash: String,
    pub data: String,
    pub hash: String,
    pub nonce: u32,
    pub timestamp: u64,
}

impl Block {
    pub fn new(
        index: u32,
        previous_hash: String,
        data: String,
        difficulty: u32,
        hasher: HashFunction,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let p = pow::ProofOfWork { data, difficulty };
        let (h, nonce) = pow::mine(&p, hasher);
        Ok(Block {
            index,
            previous_hash,
            data: p.data,
            hash: h,
            timestamp: now_unix()?,
            nonce,
        })
    }
}

pub struct Blockchain {
    pub blocks: Vec<Block>,
    pub difficulty: u32,
}

impl Blockchain {
    pub fn new(
        difficulty: u32,
        hasher: HashFunction,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let genesis_block = Block::new(
            0,
            String::new(),
            String::from("Genesis Block"),
            difficulty,
            hasher,
        );
        Ok(Self {
            difficulty,
            blocks: vec![genesis_block?],
        })
    }

    pub fn add_block(
        &mut self,
        data: &str,
        hasher: HashFunction,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let previous_block = match self.blocks.last() {
            Some(pb) => pb,
            None => return Err("Invalid state: The blockchain is empty.".into()),
        };
        self.blocks.push(Block::new(
            previous_block.index + 1,
            previous_block.hash.clone(),
            data.to_string(),
            self.difficulty,
            hasher,
        )?);
        Ok(())
    }
}
