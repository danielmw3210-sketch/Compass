use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, Deserialize)]
pub enum NetMessage {
    Transaction(Vec<u8>),
    Ping,
}

/// Start a TCP server that listens for incoming connections
pub async fn start_server(addr: &str) {
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Compass node listening on {}", addr);

    loop {
        let (mut socket, peer) = listener.accept().await.unwrap();
        println!("Incoming connection from {}", peer);

        tokio::spawn(async move {
            let mut buf = vec![0u8; 1024];
            match socket.read(&mut buf).await {
                Ok(n) if n > 0 => {
                    println!("Received {} bytes: {:?}", n, &buf[..n]);
                }
                Ok(_) => println!("Connection closed by peer"),
                Err(e) => println!("Failed to read from socket; err = {:?}", e),
            }
        });
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