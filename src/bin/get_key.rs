use libp2p::identity::Keypair;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let key = Keypair::generate_ed25519();

    let secret = key.to_protobuf_encoding()?;
    println!("{:?}", secret);

    Ok(())
}
