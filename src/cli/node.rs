use clap::Subcommand;

#[derive(Subcommand)]
pub enum NodeCommands {
    Start {
        #[arg(long)]
        rpc_port: Option<u16>,
        #[arg(long)]
        p2p_port: Option<u16>,
        #[arg(long)]
        db_path: Option<String>,
        #[arg(long)]
        peer: Option<String>,
        #[arg(long, default_value = "false")]
        ephemeral: bool,
    },
    Status,
    Peers,
    Wipe {
        #[arg(long)]
        db_path: Option<String>,
    }
}

pub async fn handle_node_command(cmd: NodeCommands) {
    match cmd {
        NodeCommands::Start { .. } => {
            // Handled in main.rs
        }
        NodeCommands::Status => {
            let client =
                crate::client::rpc_client::RpcClient::new("http://127.0.0.1:9000".to_string());
            match client.get_node_info().await {
                Ok(info) => println!("Node Status: {:#?}", info),
                Err(e) => println!("Failed to get node status: {}", e),
            }
        }
        NodeCommands::Peers => {
            let client =
                crate::client::rpc_client::RpcClient::new("http://127.0.0.1:9000".to_string());
            match client.get_peers().await {
                Ok(peers) => {
                    println!("Connected Peers ({}):", peers.len());
                    for p in peers {
                        println!(" - {}", p);
                    }
                }
                Err(e) => println!("Failed to get peers: {}", e),
            }
        }
        NodeCommands::Wipe { .. } => {
            // Handled in main.rs
        }
    }
}
