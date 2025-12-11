pub mod handlers;
pub mod types;

use crate::chain::Chain;
use axum::{routing::post, Router};
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;

use crate::gulf_stream::CompassGulfStreamManager;
use crate::network::PeerManager;

#[derive(Clone)]
pub struct RpcState {
    pub chain: Arc<Mutex<crate::chain::Chain>>,
    pub gulf_stream: Arc<Mutex<crate::gulf_stream::CompassGulfStreamManager>>,
    pub peer_manager: Arc<Mutex<crate::network::PeerManager>>, // Added for gossip
}

pub struct RpcServer {
    state: RpcState,
    bind_addr: String,
}

impl RpcServer {
    pub fn new(
        state: RpcState,
        port: u16,
    ) -> Self {
        Self {
            state,
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
