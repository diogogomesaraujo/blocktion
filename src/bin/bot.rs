use std::{error::Error, time::Duration};

use blocktion::{state::blockchain::node_rpc_service_client::NodeRpcServiceClient, time::Poisson};
use clap::Parser;
use rand::{RngCore, rngs::StdRng, thread_rng};
use tokio::time::sleep;
use tonic::transport::Channel;

const RATE: f32 = 2.;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    node_address: String,
}

async fn gen_request(
    _poisson_distribution: &mut Poisson<StdRng>,
    _client: &mut NodeRpcServiceClient<Channel>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    todo!()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = Args::parse();

    let mut client = NodeRpcServiceClient::connect(format!("http://{}", args.node_address)).await?;

    let mut poisson_distribution = {
        let mut seed: [u8; 32] = [0u8; 32];
        thread_rng().fill_bytes(&mut seed);
        Poisson::new(RATE, &seed)
    };

    loop {
        sleep(Duration::from_secs_f32(
            poisson_distribution.time_for_next_event(),
        ))
        .await;

        gen_request(&mut poisson_distribution, &mut client).await?;
    }
}
