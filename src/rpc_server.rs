use crate::models::blockchain::block::block_height::BlockHeight;
use crate::models::blockchain::digest::{Digest, Hashable};
use crate::models::blockchain::simple::*;
use crate::models::blockchain::transaction::utxo::Utxo;
use crate::models::blockchain::transaction::Transaction;
use crate::models::channel::RPCServerToMain;
use crate::models::peer::PeerInfo;
use crate::models::state::State;
use futures::executor;
use futures::future::{self, Ready};
use std::net::IpAddr;
use std::net::SocketAddr;
use tarpc::context;

#[tarpc::service]
pub trait RPC {
    /// Returns the current block height.
    async fn block_height() -> BlockHeight;
    /// Returns info about the peers we are connected to
    async fn get_peer_info() -> Vec<PeerInfo>;
    /// Returns the digest of the latest block
    async fn head() -> Digest;
    // Clears standing for all peers, connected or not.
    async fn clear_all_standings();
    // Clears standing for ip, whether connected or not.
    async fn clear_ip_standing(ip: IpAddr);
    // Send coins.
    async fn send(send_argument: String) -> bool;
}
#[derive(Clone)]
pub struct NeptuneRPCServer {
    pub socket_address: SocketAddr,
    pub state: State,
    pub rpc_server_to_main_tx: tokio::sync::mpsc::Sender<RPCServerToMain>,
}
impl RPC for NeptuneRPCServer {
    type BlockHeightFut = Ready<BlockHeight>;
    type GetPeerInfoFut = Ready<Vec<PeerInfo>>;
    type HeadFut = Ready<Digest>;
    type ClearAllStandingsFut = Ready<()>;
    type ClearIpStandingFut = Ready<()>;
    type SendFut = Ready<bool>;

    fn block_height(self, _: context::Context) -> Self::BlockHeightFut {
        // let mut databases = executor::block_on(self.state.block_databases.lock());
        // let lookup_res = databases.latest_block_header.get(());
        let latest_block = self.state.chain.light_state.get_latest_block_header();
        future::ready(latest_block.height)
    }
    fn head(self, _: context::Context) -> Ready<Digest> {
        let latest_block = self.state.chain.light_state.get_latest_block_header();
        future::ready(latest_block.hash())
    }
    fn get_peer_info(self, _: context::Context) -> Self::GetPeerInfoFut {
        let peer_map = self
            .state
            .net
            .peer_map
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect();
        future::ready(peer_map)
    }
    fn clear_all_standings(self, _: context::Context) -> Self::ClearAllStandingsFut {
        let mut peers = self
            .state
            .net
            .peer_map
            .lock()
            .unwrap_or_else(|e| panic!("Failed to lock peer map: {}", e));

        // iterates and modifies standing field for all connected peers
        peers.iter_mut().for_each(|(_, peerinfo)| {
            peerinfo.standing.clear_standing();
        });
        executor::block_on(self.state.clear_all_standings_in_database());
        future::ready(())
    }
    fn clear_ip_standing(self, _: context::Context, ip: IpAddr) -> Self::ClearIpStandingFut {
        let mut peers = self
            .state
            .net
            .peer_map
            .lock()
            .unwrap_or_else(|e| panic!("Failed to lock peer map: {}", e));
        peers.iter_mut().for_each(|(socketaddr, peerinfo)| {
            if socketaddr.ip() == ip {
                peerinfo.standing.clear_standing();
            }
        });
        //Also clears this IP's standing in database, whether it is connected or not.
        executor::block_on(self.state.clear_ip_standing_in_database(ip));
        future::ready(())
    }
    fn send(self, _ctx: context::Context, send_argument: String) -> Self::SendFut {
        let wallet = SimpleWallet::new();

        let span = tracing::debug_span!("Constructing transaction objects");
        let _enter = span.enter();

        tracing::debug!(?wallet.public_key);

        // 1. Parse
        let txs = tracing::debug_span!("Parsing TxSpec")
            .in_scope(|| serde_json::from_str::<Vec<Utxo>>(&send_argument))
            .unwrap();

        // 2. Build transaction objects.
        // We apply the strategy of using all UTXOs for the wallet as input and transfer any surplus back to our wallet.
        let dummy_transactions = txs
            .iter()
            .map(|tx| -> Transaction {
                let balance: Amount = wallet.get_balance();

                Transaction::new(
                    wallet.get_all_utxos(),
                    vec![
                        // the requested transfer
                        Utxo::new(tx.amount, tx.public_key),
                        // transfer the remainder to ourself
                        Utxo::new(balance - tx.amount, wallet.public_key),
                    ],
                    &wallet,
                )
            })
            .collect::<Vec<_>>();

        // 4. Send transaction message to main
        let response = executor::block_on(
            self.rpc_server_to_main_tx
                .send(RPCServerToMain::Send(dummy_transactions)),
        );

        // 5. Send acknowledgement to client.
        future::ready(response.is_ok())
    }
}
#[cfg(test)]
mod rpc_server_tests {
    use super::*;
    use crate::{
        config_models::network::Network, models::peer::PeerSanctionReason,
        rpc_server::NeptuneRPCServer, tests::shared::get_genesis_setup, RPC_CHANNEL_CAPACITY,
    };
    use anyhow::Result;
    use std::{
        collections::HashMap,
        net::{IpAddr, Ipv4Addr, SocketAddr},
        sync::MutexGuard,
    };
    use tracing_test::traced_test;

    #[traced_test]
    #[tokio::test]
    async fn clear_ip_standing_test() -> Result<()> {
        // Create initial conditions
        let (_peer_broadcast_tx, _from_main_rx_clone, _to_main_tx, mut _to_main_rx, state, _hsd) =
            get_genesis_setup(Network::Main, 2).await?;
        let peer_address_0 = state
            .net
            .peer_map
            .lock()
            .unwrap()
            .values()
            .collect::<Vec<_>>()[0]
            .connected_address;
        let peer_address_1 = state
            .net
            .peer_map
            .lock()
            .unwrap()
            .values()
            .collect::<Vec<_>>()[1]
            .connected_address;

        // sanction both
        let (standing_0, standing_1) = {
            let mut peers = state
                .net
                .peer_map
                .lock()
                .unwrap_or_else(|e| panic!("Failed to lock peer map: {}", e));
            peers.entry(peer_address_0).and_modify(|p| {
                p.standing.sanction(PeerSanctionReason::DifferentGenesis);
            });
            peers.entry(peer_address_1).and_modify(|p| {
                p.standing.sanction(PeerSanctionReason::DifferentGenesis);
            });
            let standing_0 = peers[&peer_address_0].standing;
            let standing_1 = peers[&peer_address_1].standing;
            (standing_0, standing_1)
        };

        state
            .write_peer_standing_on_increase(peer_address_0.ip(), standing_0)
            .await;
        state
            .write_peer_standing_on_increase(peer_address_1.ip(), standing_1)
            .await;

        // Verify expected initial conditions
        {
            let peer_standing_0 = state
                .get_peer_standing_from_database(peer_address_0.ip())
                .await;
            assert_ne!(0, peer_standing_0.unwrap().standing);
            assert_ne!(None, peer_standing_0.unwrap().latest_sanction);
            let peer_standing_1 = state
                .get_peer_standing_from_database(peer_address_1.ip())
                .await;
            assert_ne!(0, peer_standing_1.unwrap().standing);
            assert_ne!(None, peer_standing_1.unwrap().latest_sanction);

            // Clear standing of #0
            let (dummy_tx, _rx) =
                tokio::sync::mpsc::channel::<RPCServerToMain>(RPC_CHANNEL_CAPACITY);
            let rpc_server = NeptuneRPCServer {
                socket_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
                state: state.clone(),
                rpc_server_to_main_tx: dummy_tx,
            };
            rpc_server
                .clear_ip_standing(context::current(), peer_address_0.ip())
                .await;
        }
        // Verify expected resulting conditions in database
        {
            let peer_standing_0 = state
                .get_peer_standing_from_database(peer_address_0.ip())
                .await;
            assert_eq!(0, peer_standing_0.unwrap().standing);
            assert_eq!(None, peer_standing_0.unwrap().latest_sanction);
            let peer_standing_1 = state
                .get_peer_standing_from_database(peer_address_1.ip())
                .await;
            assert_ne!(0, peer_standing_1.unwrap().standing);
            assert_ne!(None, peer_standing_1.unwrap().latest_sanction);

            // Verify expected resulting conditions in peer map
            let peer_standing_0_from_memory =
                state.net.peer_map.lock().unwrap()[&peer_address_0].clone();
            assert_eq!(0, peer_standing_0_from_memory.standing.standing);
            let peer_standing_1_from_memory =
                state.net.peer_map.lock().unwrap()[&peer_address_1].clone();
            assert_ne!(0, peer_standing_1_from_memory.standing.standing);
        }
        Ok(())
    }
    #[traced_test]
    #[tokio::test]
    async fn clear_all_standings_test() -> Result<()> {
        // Create initial conditions
        let (_peer_broadcast_tx, _from_main_rx_clone, _to_main_tx, mut _to_main_rx, state, _hsd) =
            get_genesis_setup(Network::Main, 2).await?;
        let peer_address_0 = state
            .net
            .peer_map
            .lock()
            .unwrap()
            .values()
            .collect::<Vec<_>>()[0]
            .connected_address;
        let peer_address_1 = state
            .net
            .peer_map
            .lock()
            .unwrap()
            .values()
            .collect::<Vec<_>>()[1]
            .connected_address;

        // sanction both peers
        let (standing_0, standing_1) = {
            let mut peers: MutexGuard<HashMap<SocketAddr, PeerInfo>> = state
                .net
                .peer_map
                .lock()
                .unwrap_or_else(|e| panic!("Failed to lock peer map: {}", e));

            peers.entry(peer_address_0).and_modify(|p| {
                p.standing.sanction(PeerSanctionReason::DifferentGenesis);
            });
            peers.entry(peer_address_1).and_modify(|p| {
                p.standing.sanction(PeerSanctionReason::DifferentGenesis);
            });
            let standing_0 = peers[&peer_address_0].standing;
            let standing_1 = peers[&peer_address_1].standing;
            (standing_0, standing_1)
        };

        state
            .write_peer_standing_on_increase(peer_address_0.ip(), standing_0)
            .await;
        state
            .write_peer_standing_on_increase(peer_address_1.ip(), standing_1)
            .await;

        // Verify expected initial conditions
        {
            let peer_standing_0 = state
                .get_peer_standing_from_database(peer_address_0.ip())
                .await;
            assert_ne!(0, peer_standing_0.unwrap().standing);
            assert_ne!(None, peer_standing_0.unwrap().latest_sanction);
        }

        {
            let peer_standing_1 = state
                .get_peer_standing_from_database(peer_address_1.ip())
                .await;
            assert_ne!(0, peer_standing_1.unwrap().standing);
            assert_ne!(None, peer_standing_1.unwrap().latest_sanction);
        }

        // Clear standing of both by clearing all standings
        let (dummy_tx, _rx) = tokio::sync::mpsc::channel::<RPCServerToMain>(RPC_CHANNEL_CAPACITY);
        let rpc_server = NeptuneRPCServer {
            socket_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
            state: state.clone(),
            rpc_server_to_main_tx: dummy_tx.clone(),
        };
        rpc_server.clear_all_standings(context::current()).await;

        // Verify expected resulting conditions in database
        {
            let peer_standing_0 = state
                .get_peer_standing_from_database(peer_address_0.ip())
                .await;
            assert_eq!(0, peer_standing_0.unwrap().standing);
            assert_eq!(None, peer_standing_0.unwrap().latest_sanction);
        }

        {
            let peer_still_standing_1 = state
                .get_peer_standing_from_database(peer_address_1.ip())
                .await;
            assert_eq!(0, peer_still_standing_1.unwrap().standing);
            assert_eq!(None, peer_still_standing_1.unwrap().latest_sanction);
        }

        // Verify expected resulting conditions in peer map
        {
            let peer_standing_0_from_memory =
                state.net.peer_map.lock().unwrap()[&peer_address_0].clone();
            assert_eq!(0, peer_standing_0_from_memory.standing.standing);
        }

        {
            let peer_still_standing_1_from_memory =
                state.net.peer_map.lock().unwrap()[&peer_address_1].clone();
            assert_eq!(0, peer_still_standing_1_from_memory.standing.standing);
        }

        Ok(())
    }
}
