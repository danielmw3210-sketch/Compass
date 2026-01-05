//! Account System Module for Compass v2.0
//! 
//! This module implements the account-based state model with:
//! - Human-readable account names
//! - Password-based authentication
//! - BIP39 backup key recovery
//! - Cross-layer balance tracking

pub mod types;
pub mod store;
pub mod balance;
pub mod auth;
pub mod recovery;

pub use types::{Account, AccountType, AccountId};
pub use store::AccountStore;
pub use balance::BalanceStore;
pub use auth::AuthorizationModel;
pub use recovery::RecoveryKey;
