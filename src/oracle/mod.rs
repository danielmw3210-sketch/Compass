pub mod service;
pub mod types;
pub mod chains;
pub mod consensus;
pub mod monitor;
pub mod registry; // v2.0 oracle registry with staking
pub mod attestation; // v2.1 multi-signature attestation for decentralization

pub use service::OracleService;
pub use types::OracleConfig;
pub use attestation::{AttestationManager, AttestationType, PendingAttestation};
