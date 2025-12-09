// src/gulf_stream/mod.rs

pub mod transaction;
pub mod validator;
pub mod manager;
pub mod stats;
pub mod utils;

// Re‑export commonly used types so you can `use gulf_stream::...` in main.rs
pub use transaction::{TransactionStatus, CompassGulfStreamTransaction, TransactionObject};
pub use validator::ValidatorSlot;
pub use manager::{CompassGulfStreamManager, start_background_tasks};
pub use stats::{GulfStreamStats, QueueSizes};
pub use utils::{now_ms, hex_prefix};