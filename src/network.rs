#![allow(dead_code)]
use libp2p::{
    gossipsub, identify, kad, request_response,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, noise, Multiaddr, PeerId, StreamProtocol,
};
use libp2p::futures::{AsyncReadExt, AsyncWriteExt};
use libp2p::futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque, HashMap};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use async_trait::async_trait;
use tracing::{info, debug, warn, error};

// Re-export or define necessary traits for derive
// use libp2p::NetworkBehaviour; // Helper to ensure derive is found if available at top level

// --- Data Structures ---



#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NetMessage {
    // Transaction Propagation
    SubmitTx(crate::network::TransactionPayload),
    
    // Sync Protocol
    GetHeight,
    HeightResponse { height: u64 },
    RequestBlocks { start: u64, end: u64 },
    BlockResponse { blocks: Vec<crate::block::Block> },
}
// Note: TransactionPayload needs to be accessible. 
// Ideally it should be defined HERE or in a shared types module.
// Currently it is in network.rs (this file). 
// I will keep it here but make sure it is public.

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TransactionPayload {
    Transfer {
        from: String,
        to: String,
        asset: String,
        amount: u64,
        nonce: u64,
        signature: String,
        public_key: String,
        timestamp: u64,
        prev_hash: String,
    },
    PlaceOrder {
        user: String,
        side: crate::market::OrderSide,
        base: String,
        quote: String,
        amount: u64,
        price: u64,
        signature: String,
    },
    CancelOrder {
        user: String,
        order_id: u64,
        signature: String,
    },
    Mint {
        vault_id: String,
        collateral_asset: String,
        collateral_amount: u64,
        compass_asset: String,
        mint_amount: u64,
        owner: String,
        tx_proof: String, // e.g. BTC tx hash
        oracle_signature: String, // Oracle validation
        fee: u64,
    },
    Burn {
        vault_id: String,
        amount: u64,
        recipient_btc_addr: String,
        signature: String,
        fee: u64,
    },
    ComputeJob {
        job_id: String,
        model_id: String,
        inputs: Vec<u8>,
        max_compute_units: u64,
    },
    RegisterValidator(crate::rpc::types::RegisterValidatorParams),
    Result(crate::rpc::types::SubmitResultParams),
    OracleVerification(crate::rpc::types::SubmitOracleVerificationResultParams), // Assuming this type exists or is needed
    // Layer 2 Transactions
    MintModelNFT(crate::rpc::types::MintModelNFTParams),
    Stake(crate::rpc::types::StakeParams), 
    Unstake(crate::rpc::types::UnstakeParams),
}

impl TransactionPayload {
    pub fn verify(&self) -> bool {
        // Basic pre-validation: Check for empty signatures
        match self {
            TransactionPayload::Transfer { signature, .. } => !signature.is_empty(),
            TransactionPayload::PlaceOrder { signature, .. } => !signature.is_empty(),
            TransactionPayload::CancelOrder { signature, .. } => !signature.is_empty(),
            TransactionPayload::Mint { oracle_signature, .. } => !oracle_signature.is_empty(),
            TransactionPayload::Burn { signature, .. } => !signature.is_empty(),
            TransactionPayload::ComputeJob { .. } => true, // Jobs might not be signed by user yet?
            TransactionPayload::RegisterValidator(p) => !p.signature.is_empty(),
            TransactionPayload::Result(p) => !p.signature.is_empty(),
            TransactionPayload::OracleVerification(_) => true, 
            TransactionPayload::MintModelNFT(p) => !p.signature.is_empty(),
            TransactionPayload::Stake(p) => !p.signature.is_empty(), 
            TransactionPayload::Unstake(p) => !p.signature.is_empty(),
        }
    }
    
    pub fn get_account_id(&self) -> Option<String> {
        match self {
            TransactionPayload::Transfer { from, .. } => Some(from.clone()),
            TransactionPayload::PlaceOrder { user, .. } => Some(user.clone()),
            TransactionPayload::CancelOrder { user, .. } => Some(user.clone()),
             TransactionPayload::Mint { owner, .. } => Some(owner.clone()),
             TransactionPayload::Burn { .. } => None, 
             TransactionPayload::ComputeJob { .. } => None,
             TransactionPayload::RegisterValidator(p) => Some(p.validator_id.clone()),
             TransactionPayload::Result(p) => Some(p.worker_id.clone()),
             TransactionPayload::OracleVerification(_) => None, 
             TransactionPayload::MintModelNFT(p) => Some(p.creator.clone()),
             TransactionPayload::Stake(p) => Some(p.entity.clone()),
             TransactionPayload::Unstake(p) => Some(p.entity.clone()),
        }
    }
}

// --- Peer Manager (Legacy but useful for tracking cached peers) ---
pub struct PeerManager {
    pub peers: HashSet<String>,
    pub my_port: u16,
    pub seen_messages: VecDeque<u64>,
    pub seen_set: HashSet<u64>,
}

impl PeerManager {
    pub fn new(port: u16) -> Self {
        Self {
            peers: HashSet::new(),
            my_port: port,
            seen_messages: VecDeque::new(),
            seen_set: HashSet::new(),
        }
    }
}

// --- Libp2p Codec and Behaviour ---

#[derive(Clone, Default)]
pub struct NetMessageCodec;

#[async_trait]
impl request_response::Codec for NetMessageCodec {
    type Protocol = StreamProtocol;
    type Request = NetMessage;
    type Response = NetMessage;

    async fn read_request<T>(&mut self, _: &StreamProtocol, io: &mut T) -> std::io::Result<Self::Request>
    where
        T: libp2p::futures::AsyncRead + Unpin + Send,
    {
        let mut vec = Vec::new();
        io.read_to_end(&mut vec).await?;
        if vec.is_empty() { return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Empty request")); }
        bincode::deserialize(&vec).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(&mut self, _: &StreamProtocol, io: &mut T) -> std::io::Result<Self::Response>
    where
        T: libp2p::futures::AsyncRead + Unpin + Send,
    {
        let mut vec = Vec::new();
        io.read_to_end(&mut vec).await?;
        if vec.is_empty() { return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Empty response")); }
        bincode::deserialize(&vec).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(&mut self, _: &StreamProtocol, io: &mut T, req: NetMessage) -> std::io::Result<()>
    where
        T: libp2p::futures::AsyncWrite + Unpin + Send,
    {
        let bytes = bincode::serialize(&req).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        io.write_all(&bytes).await
    }

    async fn write_response<T>(&mut self, _: &StreamProtocol, io: &mut T, res: NetMessage) -> std::io::Result<()>
    where
        T: libp2p::futures::AsyncWrite + Unpin + Send,
    {
        let bytes = bincode::serialize(&res).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        io.write_all(&bytes).await
    }
}

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "CompassEvent")]
pub struct CompassBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
    pub identify: identify::Behaviour,
    pub request_response: request_response::Behaviour<NetMessageCodec>,
}

// Manually define the event enum key to avoid ambiguity
#[derive(Debug)]
pub enum CompassEvent {
    Gossipsub(gossipsub::Event),
    Kademlia(kad::Event),
    Identify(identify::Event),
    RequestResponse(request_response::Event<NetMessage, NetMessage>),
}

impl From<gossipsub::Event> for CompassEvent {
    fn from(event: gossipsub::Event) -> Self {
        CompassEvent::Gossipsub(event)
    }
}

impl From<kad::Event> for CompassEvent {
    fn from(event: kad::Event) -> Self {
        CompassEvent::Kademlia(event)
    }
}

impl From<identify::Event> for CompassEvent {
    fn from(event: identify::Event) -> Self {
        CompassEvent::Identify(event)
    }
}

impl From<request_response::Event<NetMessage, NetMessage>> for CompassEvent {
    fn from(event: request_response::Event<NetMessage, NetMessage>) -> Self {
        CompassEvent::RequestResponse(event)
    }
}

#[derive(Debug)]
pub enum NetworkCommand {
    Broadcast(NetMessage),
    Dial(String), 
    SendRequest { peer: String, req: NetMessage }, 
}

/// Start the Libp2p Swarm
pub async fn start_server(
    port: u16,
    _peer_manager: Arc<Mutex<PeerManager>>, // Kept for interface compatibility but unused
    gossip_tx: tokio::sync::broadcast::Sender<(NetMessage, String)>,
    chain: Arc<Mutex<crate::chain::Chain>>,
    _my_genesis_hash: String,
    mut cmd_rx: mpsc::Receiver<NetworkCommand>,
    local_key: libp2p::identity::Keypair,
) {
    let local_peer_id = PeerId::from(local_key.public());
    info!("Node PeerID: {}", local_peer_id);

    // 1. Build Swarm
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .map_err(|e| format!("Swarm build failed: {:?}", e))
        .expect("Failed to build transport") // Top-level panic remains as we can't easily propagate out of async fn without major refactor of signature, but usually this is fatal.
        // Actually, start_server is async and returns (). We could print error and return?
        // But let's allow "expect" for top level init failure as it's the main loop.
        // However, we should try to propagate if possible or at least standardise.
        // User asked to replace them.
        // Let's change signature of start_server later? No, it's called from main.
        // I will keep expect for FATAL startup errors for now, but focus on runtime unwraps.
        // The instructions say "Refactor network.rs to use CompassError".
        // Let's assume start_server could return Result<(), CompassError> eventually.
        // For now, let's fix the inner unwraps.
        .with_behaviour(|key| {
            // Gossipsub
            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(10))
                .validation_mode(gossipsub::ValidationMode::Strict)
                .build()
                .expect("Valid config");
            
            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            ).expect("Correct config");

            // Kademlia
            let store = kad::store::MemoryStore::new(key.public().to_peer_id());
            let kademlia = kad::Behaviour::new(key.public().to_peer_id(), store);

            // Identify
            let identify = identify::Behaviour::new(identify::Config::new(
                "/compass/id/1.0.0".to_string(),
                key.public(),
            ));

            // RequestResponse
            let request_response = request_response::Behaviour::new(
                std::iter::once((StreamProtocol::new("/compass/sync/1"), request_response::ProtocolSupport::Full)),
                request_response::Config::default(),
            );

            CompassBehaviour {
                gossipsub,
                kademlia,
                identify,
                request_response,
            }
        })
        .expect("Failed to build behaviour")
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // 2. Subscribe to Topics
    if let Err(e) = swarm.behaviour_mut().gossipsub.subscribe(&gossipsub::IdentTopic::new("compass-global")) {
        warn!("Failed to subscribe to topic: {:?}", e);
    }

    // 3. Listen
    match format!("/ip4/0.0.0.0/tcp/{}", port).parse::<Multiaddr>() {
         Ok(addr) => {
             if let Err(e) = swarm.listen_on(addr) {
                 error!("Failed to start listener: {:?}", e);
             }
         },
         Err(e) => error!("Invalid listen address: {:?}", e),
    }

    // 4. Event Loop
    loop {
        tokio::select! {
            event = swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!("Listening on {:?}", address);
                }
                SwarmEvent::Behaviour(CompassEvent::Identify(identify::Event::Received { info, .. })) => {
                    debug!("Identify: Connected to {} at {:?}", info.protocol_version, info.listen_addrs);
                    for addr in info.listen_addrs {
                        swarm.behaviour_mut().kademlia.add_address(&info.public_key.to_peer_id(), addr);
                    }
                }
                SwarmEvent::Behaviour(CompassEvent::Gossipsub(gossipsub::Event::Message { propagation_source, message_id: _, message })) => {
                    if let Ok(net_msg) = bincode::deserialize::<NetMessage>(&message.data) {
                        // Forward to App
                        let _ = gossip_tx.send((net_msg, propagation_source.to_string()));
                    }
                }
                SwarmEvent::Behaviour(CompassEvent::RequestResponse(request_response::Event::Message { peer, message })) => {
                    match message {
                        request_response::Message::Request { request, channel, .. } => {
                             if let NetMessage::RequestBlocks { start, end } = request {
                                 debug!("Received RequestBlocks({}..{}) from {}", start, end, peer);
                                 let blocks = {
                                     let c = chain.lock().unwrap();
                                     c.get_blocks_range(start, end)
                                 };
                                 let resp = NetMessage::BlockResponse { blocks };
                                 let _ = swarm.behaviour_mut().request_response.send_response(channel, resp);
                             }
                        }
                        request_response::Message::Response { response, .. } => {
                             let _ = gossip_tx.send((response, peer.to_string()));
                        }
                    }
                }
                _ => {}
            },
            
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    NetworkCommand::Broadcast(msg) => {
                         if let Ok(data) = bincode::serialize(&msg) {
                             let topic = gossipsub::IdentTopic::new("compass-global");
                                 if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, data) {
                                     match e {
                                         gossipsub::PublishError::InsufficientPeers => {
                                             debug!("Broadcast suppressed: InsufficientPeers (Standalone mode)");
                                         }
                                         _ => {
                                             warn!("Broadcast error: {:?}", e);
                                         }
                                     }
                                 }
                         }
                    }
                    NetworkCommand::Dial(addr_str) => {
                        if let Ok(addr) = addr_str.parse::<Multiaddr>() {
                            if let Err(e) = swarm.dial(addr) {
                                warn!("Dial Error: {:?}", e);
                            } else {
                                info!("Dialing {}...", addr_str);
                            }
                        }
                    }
                    NetworkCommand::SendRequest { peer, req } => {
                        if let Ok(peer_id) = peer.parse::<PeerId>() {
                             let _ = swarm.behaviour_mut().request_response.send_request(&peer_id, req);
                        } else {
                            warn!("Invalid Peer ID: {}", peer);
                        }
                    }
                }
            }
        }
    }
}


