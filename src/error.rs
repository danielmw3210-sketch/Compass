use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompassError {
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
    #[error("Hash mismatch: expected {0}, got {1}")]
    HashMismatch(String, String),
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Block verification failed: {0}")]
    VerificationError(String),
    #[error("Invalid state: {0}")]
    InvalidState(String),
    #[error("Missing requirement: {0}")]
    MissingMetadata(String),
    #[error("Transaction failed: {0}")]
    TransactionError(String),
    #[error("Unknown error: {0}")]
    Unknown(String),
}
