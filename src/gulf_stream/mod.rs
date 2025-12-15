// src/gulf_stream/mod.rs

pub mod manager;
pub mod stats;
pub mod transactions;
pub mod utils;
pub mod validator;

// Re‑export commonly used types so you can `use gulf_stream::...` in main.rs
pub use manager::CompassGulfStreamManager;
pub use utils::now_ms;
