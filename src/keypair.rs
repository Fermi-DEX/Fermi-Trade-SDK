use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signer};
use std::fs;

use crate::error::{Result, SdkError};
use crate::types::Pubkey;

/// A trading keypair for signing orders and cancellations.
/// Supports multiple input formats for flexibility.
pub struct TradingKeypair {
    inner: Keypair,
}

impl TradingKeypair {
    /// Load keypair from a JSON file containing a 64-byte array.
    /// Format: [secret_key_32_bytes..., public_key_32_bytes...]
    pub fn from_file(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| SdkError::Keypair(format!("Failed to read file '{}': {}", path, e)))?;

        let bytes: Vec<u8> = serde_json::from_str(&content)
            .map_err(|e| SdkError::Keypair(format!("Failed to parse JSON: {}", e)))?;

        if bytes.len() != 64 {
            return Err(SdkError::Keypair(format!(
                "Keypair must be 64 bytes, got {}",
                bytes.len()
            )));
        }

        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Self::from_bytes(&arr)
    }

    /// Create keypair from raw 64-byte array.
    /// Format: [secret_key_32_bytes, public_key_32_bytes]
    pub fn from_bytes(bytes: &[u8; 64]) -> Result<Self> {
        let keypair = Keypair::from_bytes(bytes)
            .map_err(|e| SdkError::Keypair(format!("Invalid keypair bytes: {}", e)))?;
        Ok(Self { inner: keypair })
    }

    /// Create keypair from a base58-encoded secret key (32 bytes).
    /// The public key is derived from the secret key.
    pub fn from_base58_secret(secret_b58: &str) -> Result<Self> {
        let secret_bytes = bs58::decode(secret_b58)
            .into_vec()
            .map_err(|e| SdkError::Keypair(format!("Invalid base58: {}", e)))?;

        if secret_bytes.len() != 32 {
            return Err(SdkError::Keypair(format!(
                "Secret key must be 32 bytes, got {}",
                secret_bytes.len()
            )));
        }

        let mut secret_arr = [0u8; 32];
        secret_arr.copy_from_slice(&secret_bytes);

        let secret = SecretKey::from_bytes(&secret_arr)
            .map_err(|e| SdkError::Keypair(format!("Invalid secret key: {}", e)))?;
        let public = PublicKey::from(&secret);

        let mut full_bytes = [0u8; 64];
        full_bytes[..32].copy_from_slice(&secret_arr);
        full_bytes[32..].copy_from_slice(public.as_bytes());

        let keypair = Keypair::from_bytes(&full_bytes)
            .map_err(|e| SdkError::Keypair(format!("Failed to create keypair: {}", e)))?;

        Ok(Self { inner: keypair })
    }

    /// Generate a new random keypair (useful for testing).
    pub fn generate() -> Self {
        let mut csprng = rand::rngs::OsRng {};
        let keypair = Keypair::generate(&mut csprng);
        Self { inner: keypair }
    }

    /// Get the public key as a Pubkey.
    pub fn pubkey(&self) -> Pubkey {
        Pubkey::new_from_array(self.inner.public.to_bytes())
    }

    /// Get the public key as a base58 string.
    pub fn pubkey_string(&self) -> String {
        bs58::encode(self.inner.public.to_bytes()).into_string()
    }

    /// Get the raw public key bytes.
    pub fn pubkey_bytes(&self) -> [u8; 32] {
        self.inner.public.to_bytes()
    }

    /// Sign a message and return the signature bytes.
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.inner.sign(message).to_bytes()
    }

    /// Sign a message and return the signature as hex string.
    pub fn sign_hex(&self, message: &[u8]) -> String {
        hex::encode(self.sign(message))
    }
}

impl std::fmt::Debug for TradingKeypair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TradingKeypair")
            .field("pubkey", &self.pubkey_string())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_sign() {
        let keypair = TradingKeypair::generate();
        let message = b"test message";
        let signature = keypair.sign(message);
        assert_eq!(signature.len(), 64);
    }

    #[test]
    fn test_pubkey_string() {
        let keypair = TradingKeypair::generate();
        let pubkey_str = keypair.pubkey_string();
        // Base58 encoded 32 bytes should be 43-44 characters
        assert!(pubkey_str.len() >= 32 && pubkey_str.len() <= 44);
    }
}
