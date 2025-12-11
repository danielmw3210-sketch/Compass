use clap::Subcommand;

#[derive(Subcommand)]
pub enum NodeCommands {
    Start {
        #[arg(long, default_value_t = 8899)]
        rpc_port: u16,
        #[arg(long)]
        peer: Option<String>,
    },
    Status,
    Peers,
}

pub async fn handle_node_command(cmd: NodeCommands) {
    match cmd {
        NodeCommands::Start { .. } => {
            // Handled in main.rs
        },
        NodeCommands::Status => {
            let client = crate::client::rpc_client::RpcClient::new("http://127.0.0.1:8899".to_string());
            match client.get_node_info().await {
                Ok(info) => println!("Node Status: {:#?}", info),
                Err(e) => println!("Failed to get node status: {}", e),
            }
        },
        NodeCommands::Peers => {
             let client = crate::client::rpc_client::RpcClient::new("http://127.0.0.1:8899".to_string());
             match client.get_peers().await {
                 Ok(peers) => {
                     println!("Connected Peers ({}):", peers.len());
                     for p in peers {
                         println!(" - {}", p);
                     }
                 },
                 Err(e) => println!("Failed to get peers: {}", e),
             }
        }
    }
}
