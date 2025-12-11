// src/gulf_stream/mod.rs

pub mod manager;
pub mod stats;
pub mod transactions;
pub mod utils;
pub mod validator;

// Re‑export commonly used types so you can `use gulf_stream::...` in main.rs
pub use manager::CompassGulfStreamManager;
pub use stats::{GulfStreamStats, QueueSizes};
pub use transactions::{CompassGulfStreamTransaction, TransactionObject, TransactionStatus};
pub use utils::{hex_prefix, now_ms};
pub use validator::ValidatorSlot;
