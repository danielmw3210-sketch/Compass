pub mod types;
pub mod handlers;

use axum::{Router, routing::post};
use std::sync::{Arc, Mutex};
use crate::chain::Chain;
use tower_http::cors::CorsLayer;

use crate::network::PeerManager;
use crate::gulf_stream::CompassGulfStreamManager;

#[derive(Clone)]
pub struct RpcState {
    pub chain: Arc<Mutex<Chain>>,
    pub peer_manager: Arc<Mutex<PeerManager>>,
    pub gulf_stream: Arc<Mutex<CompassGulfStreamManager>>,
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
        port: u16
    ) -> Self {
        Self {
            state: RpcState { chain, peer_manager, gulf_stream },
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
