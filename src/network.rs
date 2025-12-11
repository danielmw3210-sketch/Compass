use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use crate::market::OrderSide;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TransactionPayload {
    Transfer {
        from: String,
        to: String,
        asset: String,
        amount: u64,
        signature: String, // Hex encoded
    },
    PlaceOrder {
        user: String,
        side: OrderSide,
        base: String,
        quote: String,
        amount: u64,
        price: u64,
        signature: String,
    },
    // New types for Gulf Stream
    Mint(crate::rpc::types::SubmitMintParams),
    Burn(crate::rpc::types::SubmitBurnParams),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NetMessage {
    SubmitTx(TransactionPayload),
    Transaction(Vec<u8>), // Legacy raw bytes
    Ping,
    Pong,
    Handshake { port: u16 }, // Tell peer our listening port
    // Sync Messages
    RequestBlocks { start_height: u64, end_height: u64 },
    SendBlocks(Vec<crate::block::Block>),
    // Gossip
    NewPeer { addr: String },
}

pub struct PeerManager {
    pub peers: HashSet<String>, // "ip:port"
    pub my_port: u16,
}

impl PeerManager {
    pub fn new(port: u16) -> Self {
        Self {
            peers: HashSet::new(),
            my_port: port,
        }
    }

    pub fn add_peer(&mut self, peer_addr: String) {
        if !self.peers.contains(&peer_addr) {
            println!("New Peer Added: {}", peer_addr);
            self.peers.insert(peer_addr);
        }
    }
}

/// Start a TCP server that listens for incoming connections
pub async fn start_server(
    port: u16, 
    peer_manager: Arc<Mutex<PeerManager>>,
    gossip_tx: tokio::sync::broadcast::Sender<NetMessage>,
    chain: Arc<Mutex<crate::chain::Chain>>
) {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("Compass P2P Node listening on {}", addr);

    loop {
        let (mut socket, peer_addr) = listener.accept().await.unwrap();
        let peer_manager = peer_manager.clone();
        let gossip_tx = gossip_tx.clone();
        let chain = chain.clone();

        tokio::spawn(async move {
            let mut buf = vec![0u8; 1024 * 1024]; // 1MB buffer for larger messages (blocks)
            let n = match socket.read(&mut buf).await {
                Ok(n) if n > 0 => n,
                _ => return, 
            };

            let received_data = &buf[..n];
            if let Ok(msg) = bincode::deserialize::<NetMessage>(received_data) {
                // Check if it's a RequestBlocks message
                if let NetMessage::RequestBlocks { start_height, end_height } = msg {
                    println!("Received RequestBlocks({}..{}) from {}", start_height, end_height, peer_addr);
                    
                    let blocks = {
                        let c_lock = chain.lock().unwrap();
                        c_lock.get_blocks_range(start_height, end_height)
                    };
                    
                    // Respond directly
                    let resp = NetMessage::SendBlocks(blocks);
                    let resp_bytes = bincode::serialize(&resp).unwrap();
                    let _ = socket.write_all(&resp_bytes).await;
                    // Dont forward execution
                    return;
                }

                // Handle Handshake
                if let NetMessage::Handshake { port } = msg {
                     // ... logic reused ...
                     let peer_ip = peer_addr.ip().to_string();
                     let full_peer_addr = format!("{}:{}", peer_ip, port);
                     {
                         let mut pm = peer_manager.lock().unwrap();
                         pm.add_peer(full_peer_addr.clone());
                     }
                }
                
                // Gossip / Process (Send to main channel)
                let _ = gossip_tx.send(msg);
            } else {
                println!("Failed to deserialize message from {}", peer_addr);
            }
        });
    }
}

/// Broadcast a message to all known peers
pub async fn broadcast_message(
    peer_manager: Arc<Mutex<PeerManager>>,
    msg: NetMessage
) {
    let peers: Vec<String> = {
        let pm = peer_manager.lock().unwrap();
        pm.peers.iter().cloned().collect()
    };
    
    let serialized = bincode::serialize(&msg).unwrap();

    for peer in peers {
        let msg_bytes = serialized.clone();
        tokio::spawn(async move {
             if let Ok(mut stream) = TcpStream::connect(&peer).await {
                 let _ = stream.write_all(&msg_bytes).await;
             } else {
                 println!("Failed to connect to peer {}", peer);
                 // Remove peer?
             }
        });
    }
}

/// Connect to a peer and handshake
pub async fn connect_to_peer(peer_addr: &str, my_port: u16, peer_manager: Arc<Mutex<PeerManager>>) {
    if let Ok(mut stream) = TcpStream::connect(peer_addr).await {
        println!("Connected to bootstrap peer {}", peer_addr);
        
        // Add to list
        {
            let mut pm = peer_manager.lock().unwrap();
            pm.add_peer(peer_addr.to_string());
        }

        // Send Handshake
        let msg = NetMessage::Handshake { port: my_port };
        let bytes = bincode::serialize(&msg).unwrap();
        let _ = stream.write_all(&bytes).await;
    } else {
        println!("Failed to connect to bootstrap peer {}", peer_addr);
    }
}

/// Connect to a peer and send a message
pub async fn connect_and_send(addr: &str, msg: NetMessage) {
    match TcpStream::connect(addr).await {
        Ok(mut stream) => {
            println!("Connected to {}", addr);
            let data = bincode::serialize(&msg).unwrap();
            if let Err(e) = stream.write_all(&data).await {
                println!("Failed to send message: {:?}", e);
            }
        }
        Err(e) => println!("Failed to connect: {:?}", e),
    }
}