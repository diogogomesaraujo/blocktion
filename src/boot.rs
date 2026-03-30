use crate::{behaviour::MyBehaviour, rpc::Rpc};
use async_trait::async_trait;
use libp2p::{
    Multiaddr, PeerId, StreamProtocol, Swarm, SwarmBuilder, identify,
    identity::Keypair,
    kad::{self, Mode},
    noise, ping, tcp, yamux,
};
use std::{error::Error, str::SplitWhitespace, time::Duration};
use tracing::info;

pub struct BootNode(Multiaddr);

pub enum RpcAction {
    Ping,
    RoutingTable,
}

impl BootNode {
    pub fn new(address: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(Self(address.parse::<Multiaddr>()?))
    }
}

#[async_trait]
impl Rpc for BootNode {
    type RpcAction = RpcAction;

    fn action_from_str(action_text: &str) -> Option<Self::RpcAction> {
        match action_text {
            "PING" => Some(RpcAction::Ping),
            "ROUTING_TABLE" => Some(RpcAction::RoutingTable),
            _ => None,
        }
    }

    async fn init(
        self,
        ipfs_proto_name: StreamProtocol,
        key: Keypair,
    ) -> Result<Swarm<MyBehaviour>, Box<dyn Error + Send + Sync>> {
        let mut swarm = SwarmBuilder::with_existing_identity(key)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_dns()?
            .with_behaviour(|key| {
                let local_id = key.public().to_peer_id();

                let mut kad_cfg = kad::Config::new(ipfs_proto_name.clone());
                kad_cfg.set_query_timeout(Duration::from_secs(60));
                kad_cfg.set_periodic_bootstrap_interval(Some(Duration::from_secs(300)));

                let store = kad::store::MemoryStore::new(key.public().to_peer_id());

                let kad = kad::Behaviour::with_config(local_id, store, kad_cfg);

                let ping = ping::Behaviour::new(
                    ping::Config::new()
                        .with_interval(Duration::from_secs(10))
                        .with_timeout(Duration::from_secs(3)),
                );

                let identify = identify::Behaviour::new(identify::Config::new(
                    ipfs_proto_name.to_string(),
                    key.public(),
                ));

                Ok(MyBehaviour {
                    kad,
                    ping,
                    identify,
                })
            })?
            .build();

        swarm.behaviour_mut().kad.set_mode(Some(Mode::Server));
        swarm.listen_on(self.0)?;
        // swarm
        //     .behaviour_mut()
        //     .kad
        //     .get_closest_peers(swarm.local_peer_id());

        Ok(swarm)
    }

    fn match_action(
        args: &mut SplitWhitespace,
        swarm: &mut Swarm<MyBehaviour>,
        rpc: RpcAction,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        match rpc {
            RpcAction::Ping => {
                let address = Self::arg_parse(args)?.parse::<Multiaddr>()?;
                swarm.dial(address)?;
            }

            RpcAction::RoutingTable => {
                info!(
                    "Current state of the routing table: {:?}",
                    swarm.connected_peers().collect::<Vec<&PeerId>>(),
                );
            }
        }

        Ok(())
    }
}
