use libp2p::{StreamProtocol, identity::Keypair};
use p2p_auction::{boot::BootNode, rpc::Rpc};
use std::error::Error;
use tokio::io::{BufReader, stdin};

const IPFS_PROTO_NAME: StreamProtocol = StreamProtocol::new("/p2p-auction/1.0.0");

const BOOT_NODE: [u8; 68] = [
    8, 1, 18, 64, 245, 197, 207, 72, 103, 164, 20, 18, 54, 57, 106, 162, 23, 140, 17, 177, 222,
    233, 223, 239, 185, 246, 146, 232, 64, 142, 228, 28, 79, 207, 135, 148, 129, 197, 206, 203,
    164, 3, 214, 28, 120, 168, 113, 89, 171, 87, 169, 101, 166, 227, 156, 148, 57, 216, 180, 155,
    91, 80, 253, 174, 8, 219, 229, 95,
];

const BOOT_NODE_ADDR: &str = "/ip4/127.0.0.1/tcp/63358";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    tracing_subscriber::fmt().try_init()?;

    let node = BootNode::new(BOOT_NODE_ADDR)?;
    let key = Keypair::from_protobuf_encoding(&BOOT_NODE)?;

    let mut i = node.init(IPFS_PROTO_NAME, key).await?;
    BootNode::run(&mut i, BufReader::new(stdin())).await?;

    Ok(())
}
