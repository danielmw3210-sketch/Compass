pub mod account; // v2.0 account-based system (must be before storage)
pub mod block;
pub mod chain;
pub mod layer2;
pub mod error;
pub mod client;
pub mod crypto;
pub mod genesis;
pub mod gulf_stream;
pub mod market;
pub mod poh_recorder;
pub mod vm;
pub mod oracle;
pub mod rpc;
pub mod storage;
pub mod vault;
pub mod layer3;
pub mod vdf;
pub mod wallet;
pub mod  worker_menu;
pub mod cli;
pub mod network;
pub mod encoding;
pub mod identity;
pub mod interactive;
pub mod trainer; // Rust Native AI
pub mod init;
pub mod node;
pub mod config;
// GUI module removed - use web interface or CLI instead
