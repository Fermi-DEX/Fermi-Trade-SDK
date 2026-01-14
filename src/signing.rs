//! Order and cancel signing for perps.
//!
//! IMPORTANT: This module uses the exact Borsh structure from
//! `sequencer_client/scripts/place_perp_order_fixed.rs` which is the
//! canonical perps order structure WITHOUT the `order_type` field.

use borsh::BorshSerialize;
use sha2::{Digest, Sha256};
use serde::Serialize;

use crate::error::{Result, SdkError};
use crate::keypair::TradingKeypair;
use crate::types::{MarginMode, MarketKind, OrderSide, PositionEffect, Pubkey, Side};

// =============================================================================
// Signing prefixes (must match server)
// =============================================================================

const SIGNED_ORDER_PREFIX: &[u8] = b"FRM_DEX_ORDER:";
const CANCEL_ORDER_PREFIX: &[u8] = b"FRM_DEX_CANCEL:";

// =============================================================================
// Borsh structures for signing (MUST match server exactly)
// =============================================================================

/// PerpOrderIntentBorsh - EXACTLY matching server structure
/// Reference: sequencer_client/scripts/place_perp_order_fixed.rs:43-59
/// NOTE: NO order_type field (spot has it, perps don't)
#[derive(Debug, Clone, BorshSerialize)]
struct PerpOrderIntentBorsh {
    order_id: u64,
    owner: Pubkey,
    side: OrderSide,
    price: u64,
    quantity: u64,
    expiry: u64,
    base_mint: Pubkey,
    quote_mint: Pubkey,
    market_kind: MarketKind,
    leverage: Option<u64>,
    position_effect: Option<PositionEffect>,
    reduce_only: bool,
    margin_mode: Option<MarginMode>,
    margin_amount: Option<u64>,
    liquidation: bool,
}

/// CancelOrderData for signing cancellations
#[derive(Debug, Clone, BorshSerialize)]
struct CancelOrderData {
    order_id: u64,
    owner: Pubkey,
    base_mint: Pubkey,
    quote_mint: Pubkey,
}

// =============================================================================
// JSON DTOs for API submission
// =============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct OrderIntentDto {
    pub order_id: u64,
    pub owner: String,
    pub side: String,
    pub price: u64,
    pub quantity: u64,
    pub expiry: u64,
    pub base_mint: String,
    pub quote_mint: String,
    pub market_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leverage: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_effect: Option<String>,
    pub reduce_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_amount: Option<u64>,
    pub liquidation: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SignedOrderRequest {
    pub intent: OrderIntentDto,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CancelOrderRequest {
    pub order_id: u64,
    pub owner: String,
    pub base_mint: String,
    pub quote_mint: String,
    pub signature: String,
}

// =============================================================================
// Signed order/cancel results
// =============================================================================

/// A signed perp order ready for submission
#[derive(Debug, Clone)]
pub struct SignedOrder {
    pub order_id: u64,
    pub request: SignedOrderRequest,
    pub owner_bytes: [u8; 32],
}

/// A signed cancel request ready for submission
#[derive(Debug, Clone)]
pub struct SignedCancel {
    pub order_id: u64,
    pub request: CancelOrderRequest,
    pub owner_bytes: [u8; 32],
}

// =============================================================================
// Signing functions
// =============================================================================

/// Sign a perp order using the exact server structure.
/// Reference: sequencer_client/scripts/place_perp_order_fixed.rs:87-110
#[allow(clippy::too_many_arguments)]
pub fn sign_perp_order(
    keypair: &TradingKeypair,
    order_id: u64,
    side: Side,
    price: u64,
    quantity: u64,
    expiry: u64,
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
    leverage: u64,
    position_effect: PositionEffect,
    margin_mode: MarginMode,
    margin_amount: Option<u64>,
    reduce_only: bool,
) -> Result<SignedOrder> {
    // 1. Build PerpOrderIntentBorsh for signing
    let perp_intent = PerpOrderIntentBorsh {
        order_id,
        owner: keypair.pubkey(),
        side: side.into(),
        price,
        quantity,
        expiry,
        base_mint: *base_mint,
        quote_mint: *quote_mint,
        market_kind: MarketKind::Perp, // ALWAYS Perp for perps SDK
        leverage: Some(leverage),
        position_effect: Some(position_effect),
        reduce_only,
        margin_mode: Some(margin_mode),
        margin_amount,
        liquidation: false,
    };

    // 2. Create signing message: PREFIX + Borsh(intent)
    let mut data = SIGNED_ORDER_PREFIX.to_vec();
    let borsh_bytes = perp_intent
        .try_to_vec()
        .map_err(|e| SdkError::Serialization(format!("Borsh serialization failed: {}", e)))?;

    tracing::debug!("Borsh serialized bytes ({} bytes): {:02x?}", borsh_bytes.len(), &borsh_bytes[..std::cmp::min(100, borsh_bytes.len())]);
    data.extend(borsh_bytes);

    // 3. Hash: SHA256(data) -> hex string -> UTF-8 bytes
    let hash = Sha256::digest(&data);
    let hex_string = hex::encode(hash);
    let message = hex_string.as_bytes();

    tracing::debug!("Order SHA256 hash: {}", hex_string);

    // 4. Sign the message bytes
    let signature = keypair.sign(message);
    let signature_hex = hex::encode(signature);
    tracing::debug!("Order signature: {}", signature_hex);

    // 5. Build the JSON request DTO
    let dto = OrderIntentDto {
        order_id,
        owner: keypair.pubkey_string(),
        side: match side {
            Side::Buy => "Buy".to_string(),
            Side::Sell => "Sell".to_string(),
        },
        price,
        quantity,
        expiry,
        base_mint: base_mint.to_string(),
        quote_mint: quote_mint.to_string(),
        market_kind: "perp".to_string(),
        leverage: Some(leverage),
        position_effect: Some(position_effect.to_string()),
        reduce_only,
        margin_mode: Some(margin_mode.to_string()),
        margin_amount,
        liquidation: false,
    };

    let request = SignedOrderRequest {
        intent: dto,
        signature: signature_hex,
    };

    Ok(SignedOrder {
        order_id,
        request,
        owner_bytes: keypair.pubkey_bytes(),
    })
}

/// Sign a cancel request.
/// Reference: sequencer_client/src/order_cancel.rs
pub fn sign_cancel(
    keypair: &TradingKeypair,
    order_id: u64,
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
) -> Result<SignedCancel> {
    // 1. Build CancelOrderData for signing
    let cancel_data = CancelOrderData {
        order_id,
        owner: keypair.pubkey(),
        base_mint: *base_mint,
        quote_mint: *quote_mint,
    };

    // 2. Create signing message: PREFIX + Borsh(cancel_data)
    let mut data = CANCEL_ORDER_PREFIX.to_vec();
    data.extend(
        cancel_data
            .try_to_vec()
            .map_err(|e| SdkError::Serialization(format!("Borsh serialization failed: {}", e)))?,
    );

    // 3. Hash: SHA256(data) -> hex string -> UTF-8 bytes
    let hash = Sha256::digest(&data);
    let hex_string = hex::encode(hash);
    let message = hex_string.as_bytes();

    // 4. Sign the message bytes
    let signature = keypair.sign(message);
    let signature_hex = hex::encode(signature);

    // 5. Build the JSON request
    let request = CancelOrderRequest {
        order_id,
        owner: keypair.pubkey_string(),
        base_mint: base_mint.to_string(),
        quote_mint: quote_mint.to_string(),
        signature: signature_hex,
    };

    Ok(SignedCancel {
        order_id,
        request,
        owner_bytes: keypair.pubkey_bytes(),
    })
}

impl SignedOrder {
    /// Convert the signed order request to JSON string
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(&self.request).map_err(|e| SdkError::Serialization(e.to_string()))
    }
}

impl SignedCancel {
    /// Convert the signed cancel request to JSON string
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(&self.request).map_err(|e| SdkError::Serialization(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_sign_perp_order() {
        let keypair = TradingKeypair::generate();
        let base_mint = Pubkey::from_str("11111111111111111111111111111112").unwrap();
        let quote_mint = Pubkey::from_str("11111111111111111111111111111113").unwrap();

        let signed = sign_perp_order(
            &keypair,
            12345,
            Side::Buy,
            185_500_000, // 185.50 with 6 decimals
            1_000_000_000, // 1.0 with 9 decimals
            1700000000,
            &base_mint,
            &quote_mint,
            10, // 10x leverage
            PositionEffect::Open,
            MarginMode::Cross,
            Some(18_550_000), // margin amount
            false,
        )
        .unwrap();

        assert_eq!(signed.order_id, 12345);
        assert!(!signed.request.signature.is_empty());
        assert_eq!(signed.request.intent.market_kind, "perp");
    }

    #[test]
    fn test_sign_cancel() {
        let keypair = TradingKeypair::generate();
        let base_mint = Pubkey::from_str("11111111111111111111111111111112").unwrap();
        let quote_mint = Pubkey::from_str("11111111111111111111111111111113").unwrap();

        let signed = sign_cancel(&keypair, 12345, &base_mint, &quote_mint).unwrap();

        assert_eq!(signed.order_id, 12345);
        assert!(!signed.request.signature.is_empty());
    }
}
