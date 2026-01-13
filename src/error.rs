use thiserror::Error;

/// SDK-specific errors
#[derive(Error, Debug)]
pub enum SdkError {
    #[error("Keypair error: {0}")]
    Keypair(String),

    #[error("Signing error: {0}")]
    Signing(String),

    #[error("Continuum connection error: {0}")]
    ContinuumConnection(String),

    #[error("Continuum submission error: {0}")]
    ContinuumSubmission(String),

    #[error("RPC error: {0}")]
    Rpc(String),

    #[error("Market not found: {0}")]
    MarketNotFound(String),

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Invalid pubkey: {0}")]
    InvalidPubkey(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Decimal conversion error: {0}")]
    DecimalConversion(String),

    #[error("Airdrop error: {0}")]
    Airdrop(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

impl From<reqwest::Error> for SdkError {
    fn from(err: reqwest::Error) -> Self {
        SdkError::Rpc(err.to_string())
    }
}

impl From<tonic::transport::Error> for SdkError {
    fn from(err: tonic::transport::Error) -> Self {
        SdkError::ContinuumConnection(err.to_string())
    }
}

impl From<tonic::Status> for SdkError {
    fn from(err: tonic::Status) -> Self {
        SdkError::ContinuumSubmission(format!("{}: {}", err.code(), err.message()))
    }
}

impl From<serde_json::Error> for SdkError {
    fn from(err: serde_json::Error) -> Self {
        SdkError::Serialization(err.to_string())
    }
}

impl From<std::io::Error> for SdkError {
    fn from(err: std::io::Error) -> Self {
        SdkError::Keypair(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, SdkError>;
