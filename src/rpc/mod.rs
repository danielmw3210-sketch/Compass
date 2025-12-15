pub mod handlers;
pub mod types;

use crate::chain::Chain;
use axum::{routing::post, Router};
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;

use crate::gulf_stream::CompassGulfStreamManager;
use crate::network::{PeerManager, NetworkCommand};
use crate::vault::VaultManager;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct RpcState {
    pub chain: Arc<Mutex<Chain>>,
    pub peer_manager: Arc<Mutex<PeerManager>>,
    pub gulf_stream: Arc<Mutex<CompassGulfStreamManager>>,
    pub vault_manager: Arc<Mutex<VaultManager>>,
    pub wallet_manager: Arc<Mutex<crate::wallet::WalletManager>>,
    pub layer2: Arc<Mutex<crate::layer2::Layer2State>>,
    pub betting_ledger: Arc<Mutex<crate::layer3::betting::BettingLedger>>,
    pub market: Arc<Mutex<crate::market::Market>>,
    pub cmd_tx: mpsc::Sender<NetworkCommand>,
    pub node_identity: String, // Public Key Hex
}

pub struct RpcServer {
    state: RpcState,
    bind_addr: String,
}

impl RpcServer {
    pub fn new(
        chain: Arc<Mutex<Chain>>,
        peer_manager: Arc<Mutex<PeerManager>>,
        gulf_stream: Arc<Mutex<CompassGulfStreamManager>>,
        vault_manager: Arc<Mutex<VaultManager>>,
        wallet_manager: Arc<Mutex<crate::wallet::WalletManager>>,
        layer2: Arc<Mutex<crate::layer2::Layer2State>>,
        betting_ledger: Arc<Mutex<crate::layer3::betting::BettingLedger>>,
        market: Arc<Mutex<crate::market::Market>>,
        cmd_tx: mpsc::Sender<NetworkCommand>,
        port: u16,
        node_identity: String,
    ) -> Self {
        Self {
            state: RpcState {
                chain,
                peer_manager,
                gulf_stream,
                vault_manager,
                wallet_manager,
                layer2,
                betting_ledger,
                market,
                cmd_tx,
                node_identity,
            },
            bind_addr: format!("0.0.0.0:{}", port),
        }
    }

    pub async fn start(self) {
        let app = Router::new()
            .route("/", post(handlers::handle_rpc_request))
            .layer(CorsLayer::permissive())
            .with_state(self.state);

        let listener = tokio::net::TcpListener::bind(&self.bind_addr)
            .await
            .expect("Failed to bind RPC server");

        println!("🌐 RPC server listening on {}", self.bind_addr);
        axum::serve(listener, app).await.expect("RPC server failed");
    }
}
// RPC server module
