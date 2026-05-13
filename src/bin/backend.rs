use blocktion::{
    state::blockchain::{LongestChainRequest, node_rpc_service_client::NodeRpcServiceClient},
    time::{Timestamp, now_unix},
};
use clap::Parser;
use priority_queue::PriorityQueue;
use std::{collections::HashMap, error::Error, sync::Arc};
use tokio::sync::RwLock;
use tonic::{Request, transport::Channel};

type Client = NodeRpcServiceClient<Channel>;
type Currency = u64;

const EXECUTE_AFTER_N_BLOCKS: u32 = 10;

struct ChainState {
    longest_chain: Vec<String>,
    last_executed: usize,
}

impl ChainState {
    async fn new(client: &mut Client) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(Self {
            longest_chain: request_longest_chain(client).await?,
            last_executed: 0,
        })
    }
}

struct BackendState {
    chain_state: ChainState,
    accounts: HashMap<String, Account>,
    auctions: HashMap<String, Auction>,
}

impl BackendState {
    async fn new(client: &mut Client) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(Self {
            chain_state: ChainState::new(client).await?,
            accounts: HashMap::new(),
            auctions: HashMap::new(),
        })
    }
}

struct Account {
    id: String,
    funds: Currency,
}

struct Auction {
    id: String,
    creator_id: String,
    bids: PriorityQueue<Bid, (Currency, usize)>,
    stop_time: Timestamp,
}

impl Auction {
    fn new(id: &str, creator_id: &str, stop_time: Timestamp) -> Self {
        Self {
            id: id.to_string(),
            creator_id: creator_id.to_string(),
            bids: PriorityQueue::new(),
            stop_time,
        }
    }

    fn push(&mut self, bid: Bid) {
        self.bids.push(bid.clone(), (bid.amount, bid.block_idx));
    }
}

#[derive(Hash, PartialEq, PartialOrd, Eq, Clone)]
struct Bid {
    from: String,
    amount: Currency,
    block_idx: usize,
    timestamp: Timestamp,
}

impl Bid {
    fn new(
        from: &str,
        amount: Currency,
        block_idx: usize,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(Self {
            from: from.to_string(),
            amount,
            block_idx,
            timestamp: now_unix()?,
        })
    }
}

impl Ord for Bid {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.amount, self.block_idx).cmp(&(other.amount, other.block_idx))
    }
}

struct Backend {
    client: Client,
    state: Arc<RwLock<BackendState>>,
}

impl Backend {
    async fn init(address: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let mut client = NodeRpcServiceClient::connect(address.to_string()).await?;
        let state = Arc::new(RwLock::new(BackendState::new(&mut client).await?));
        Ok(Self { client, state })
    }

    async fn run(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    node_address: String,

    #[arg(long)]
    port: String,
}

async fn request_longest_chain(
    client: &mut Client,
) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let longest_chain_response = client
        .longest_chain(Request::new(LongestChainRequest {}))
        .await?
        .into_inner();
    Ok(longest_chain_response.longest_chain)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = Args::parse();

    let backend = Backend::init(&args.node_address).await?;
    backend.run().await?;

    Ok(())
}
