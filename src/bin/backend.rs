use blocktion::{
    state::blockchain::{
        Block, BlockInfoRequest, CreateAccount, CreateAuction, LongestChainRequest,
        node_rpc_service_client::NodeRpcServiceClient,
    },
    time::{Timestamp, now_unix},
};
use clap::Parser;
use priority_queue::PriorityQueue;
use std::{collections::HashMap, error::Error, sync::Arc};
use std::{hash::Hash, time::Duration};
use tokio::{sync::RwLock, time::sleep};
use tonic::{Request, transport::Channel};

type Client = NodeRpcServiceClient<Channel>;
type Currency = u64;

const EXECUTE_AFTER_N_BLOCKS: usize = 0;
const START_FUNDS: usize = 1000;
const UPDATE_DELAY: Duration = Duration::from_secs(1);

#[derive(Debug)]
struct ChainState {
    longest_chain: Vec<String>,
    to_execute: usize,
}

impl ChainState {
    fn new() -> Self {
        Self {
            longest_chain: Vec::new(),
            to_execute: 0,
        }
    }
}

#[async_trait::async_trait]
trait Concurrent {
    async fn execute_block(&self, block: Block) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn execute_chain(
        &mut self,
        client: &mut Client,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn update(&mut self, client: &mut Client) -> Result<(), Box<dyn Error + Send + Sync>>;
}

#[derive(Debug)]
struct BackendState {
    chain_state: ChainState,
    accounts: HashMap<String, Account>,
    auctions: HashMap<String, Auction>,
}

impl BackendState {
    async fn new() -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(Self {
            chain_state: ChainState::new(),
            accounts: HashMap::new(),
            auctions: HashMap::new(),
        })
    }
}

#[async_trait::async_trait]
impl Concurrent for Arc<RwLock<BackendState>> {
    async fn execute_block(&self, block: Block) -> Result<(), Box<dyn Error + Send + Sync>> {
        for t in block.transactions.into_iter() {
            if let Some(t) = t.record {
                match t {
                    blocktion::state::blockchain::transaction::Record::CreateAccountRequest(
                        CreateAccount { public_key },
                    ) => {
                        self.write().await.accounts.insert(
                            public_key.clone(),
                            Account {
                                funds: START_FUNDS as Currency,
                            },
                        );
                    }
                    blocktion::state::blockchain::transaction::Record::BidRequest(
                        blocktion::state::blockchain::Bid {
                            from,
                            amount,
                            auction_id,
                        },
                    ) => {
                        if let Some(auction) = self.write().await.auctions.get_mut(&auction_id) {
                            auction.bids.push(Bid::new(&from, amount)?, amount);
                        }
                    }
                    blocktion::state::blockchain::transaction::Record::CreateAuctionRequest(
                        CreateAuction {
                            auction_id,
                            from,
                            start_amount,
                            stop_time,
                        },
                    ) => {
                        self.write().await.auctions.insert(
                            auction_id.clone(),
                            Auction::new(&auction_id, &from, stop_time, start_amount),
                        );
                    }
                }
            }
        }

        Ok(())
    }

    async fn execute_chain(
        &mut self,
        client: &mut Client,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let longest_chain = self.read().await.chain_state.longest_chain.clone();

        let len = longest_chain.len();
        let from = self.read().await.chain_state.to_execute;

        for i in from..len {
            let h = longest_chain[i].clone();

            let b = client
                .block_info(Request::new(BlockInfoRequest { hash: h.clone() }))
                .await?
                .into_inner();

            if let Some(block) = b.block {
                self.write().await.chain_state.to_execute += 1;
                self.execute_block(block).await?;
            }
        }

        Ok(())
    }

    async fn update(&mut self, client: &mut Client) -> Result<(), Box<dyn Error + Send + Sync>> {
        let node_longest_chain = client
            .longest_chain(Request::new(LongestChainRequest {}))
            .await?
            .into_inner()
            .longest_chain;

        let own_longest_chain = self.read().await.chain_state.longest_chain.clone();

        if &node_longest_chain != &own_longest_chain {
            self.write().await.chain_state.longest_chain = node_longest_chain;
            Self::execute_chain(self, client).await?;
        }

        Ok(())
    }
}

#[derive(Debug)]
struct Account {
    funds: Currency,
}

#[derive(Debug)]
struct Auction {
    id: String,
    creator_id: String,
    bids: PriorityQueue<Bid, Currency>,
    stop_time: Timestamp,
    start_amount: Currency,
}

impl Auction {
    fn new(id: &str, creator_id: &str, stop_time: Timestamp, start_amount: Currency) -> Self {
        Self {
            id: id.to_string(),
            creator_id: creator_id.to_string(),
            bids: PriorityQueue::new(),
            stop_time,
            start_amount,
        }
    }

    fn push(&mut self, bid: Bid) {
        self.bids.push(bid.clone(), bid.amount);
    }
}

#[derive(Hash, PartialEq, PartialOrd, Eq, Clone, Debug)]
struct Bid {
    from: String,
    amount: Currency,
    timestamp: Timestamp,
}

impl Bid {
    fn new(from: &str, amount: Currency) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(Self {
            from: from.to_string(),
            amount,
            timestamp: now_unix()?,
        })
    }
}

impl Ord for Bid {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.amount.cmp(&other.amount)
    }
}

#[derive(Debug)]
struct Backend {
    node_address: String,
    state: Arc<RwLock<BackendState>>,
}

impl Backend {
    async fn init(address: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let state = Arc::new(RwLock::new(BackendState::new().await?));

        Ok(Self {
            state,
            node_address: address.to_string(),
        })
    }

    async fn run(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut state = self.state.clone();
        let address = self.node_address.clone();

        tokio::spawn(async move {
            let mut client = NodeRpcServiceClient::connect(address).await.unwrap();

            loop {
                sleep(UPDATE_DELAY).await;
                if let Err(e) = state.update(&mut client).await {
                    tracing::error!("{e}");
                }

                tracing::info!("The current backend state is: {:?}", state);
            }
        });

        Ok(())
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    node_port: usize,

    #[arg(long)]
    port: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = Args::parse();

    tracing_subscriber::fmt().try_init()?;

    let mut backend = Backend::init(&format!("http://127.0.0.1:{}", args.node_port)).await?;
    backend.run().await?;

    sleep(Duration::from_secs(20)).await;

    Ok(())
}
