use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum NetMessage {
    Transaction(Vec<u8>),
    Ping,
}

// Stub server: compiles but does nothing yet
pub async fn start_server(addr: &str) {
    println!("Networking disabled. Listening would be on {}", addr);
}

// Stub client: compiles but does nothing yet
pub async fn connect_and_send(addr: &str, msg: NetMessage) {
    println!("Would send {:?} to {}", msg, addr);
}