use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// =============================================================================
// Pubkey - 32-byte public key
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize)]
pub struct Pubkey(pub [u8; 32]);

impl Pubkey {
    pub fn new_from_array(bytes: [u8; 32]) -> Self {
        Pubkey(bytes)
    }

    pub fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
}

impl Default for Pubkey {
    fn default() -> Self {
        Pubkey([0u8; 32])
    }
}

impl fmt::Display for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", bs58::encode(&self.0).into_string())
    }
}

impl FromStr for Pubkey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = bs58::decode(s)
            .into_vec()
            .map_err(|e| format!("Invalid base58: {}", e))?;
        if bytes.len() != 32 {
            return Err(format!("Expected 32 bytes, got {}", bytes.len()));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Pubkey(arr))
    }
}

impl AsRef<[u8]> for Pubkey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

// =============================================================================
// Borsh-serializable enums for signing (MUST match server exactly)
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

impl fmt::Display for OrderSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderSide::Buy => write!(f, "Buy"),
            OrderSide::Sell => write!(f, "Sell"),
        }
    }
}

/// Market kind for Borsh signing (perps only)
/// NOTE: This enum only contains Perp because this SDK is perps-only.
/// The discriminant must be 0 for Perp to match the signing scripts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum MarketKind {
    Perp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum PositionEffect {
    Open,
    Close,
}

impl fmt::Display for PositionEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PositionEffect::Open => write!(f, "open"),
            PositionEffect::Close => write!(f, "close"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum MarginMode {
    Cross,
    Isolated,
}

impl fmt::Display for MarginMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MarginMode::Cross => write!(f, "cross"),
            MarginMode::Isolated => write!(f, "isolated"),
        }
    }
}

// =============================================================================
// User-facing SDK types
// =============================================================================

/// Side of an order (user-friendly version)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

impl From<Side> for OrderSide {
    fn from(side: Side) -> Self {
        match side {
            Side::Buy => OrderSide::Buy,
            Side::Sell => OrderSide::Sell,
        }
    }
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Side::Buy => write!(f, "buy"),
            Side::Sell => write!(f, "sell"),
        }
    }
}

/// A perpetual order to be placed
#[derive(Debug, Clone)]
pub struct PerpOrder {
    pub side: Side,
    pub price: f64,
    pub quantity: f64,
    pub leverage: u64,
    pub position_effect: PositionEffect,
    pub margin_mode: MarginMode,
    pub reduce_only: bool,
}

impl Default for PerpOrder {
    fn default() -> Self {
        Self {
            side: Side::Buy,
            price: 0.0,
            quantity: 0.0,
            leverage: 1,
            position_effect: PositionEffect::Open,
            margin_mode: MarginMode::Cross,
            reduce_only: false,
        }
    }
}

/// Result of placing an order
#[derive(Debug, Clone)]
pub struct OrderResult {
    pub order_id: u64,
    pub sequence_number: u64,
    pub expected_tick: u64,
    pub tx_hash: String,
}

/// Result of cancelling an order
#[derive(Debug, Clone)]
pub struct CancelResult {
    pub order_id: u64,
    pub sequence_number: u64,
    pub expected_tick: u64,
    pub tx_hash: String,
}

/// Market information
#[derive(Debug, Clone, Deserialize)]
pub struct MarketInfo {
    pub uuid: String,
    pub base_mint: String,
    pub quote_mint: String,
    pub name: String,
    pub created_at: u64,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub base_decimals: u8,
    #[serde(default)]
    pub quote_decimals: u8,
    #[serde(default)]
    pub base_lot_size: u64,
    #[serde(default)]
    pub quote_lot_size: u64,
    #[serde(default)]
    pub price_decimals: Option<u8>,
    #[serde(default)]
    pub open_interest: Option<i128>,
}

/// A single order in the orderbook
#[derive(Debug, Clone, Deserialize)]
pub struct OrderbookEntry {
    pub order_id: u64,
    pub owner: String,
    pub price: u64,
    pub quantity: u64,
    pub side: String,
    pub expiry: u64,
}

/// Orderbook data
#[derive(Debug, Clone, Deserialize)]
pub struct Orderbook {
    pub buys: Vec<OrderbookEntry>,
    pub sells: Vec<OrderbookEntry>,
}

/// Depth data (Binance-style)
#[derive(Debug, Clone, Deserialize)]
pub struct Depth {
    #[serde(rename = "lastUpdateId")]
    pub last_update_id: u64,
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
}

/// Trade information
#[derive(Debug, Clone, Deserialize)]
pub struct Trade {
    pub buyer_owner: String,
    pub seller_owner: String,
    pub price: u64,
    pub quantity: u64,
    pub timestamp: u64,
    pub base_mint: String,
    pub quote_mint: String,
}

/// Funding event
#[derive(Debug, Clone, Deserialize)]
pub struct FundingEvent {
    pub market_id: String,
    pub timestamp: u64,
    pub interval_seconds: u64,
    pub mark_price: u64,
    pub index_price: u64,
    pub premium_rate_bps: i64,
    pub funding_rate_bps: i64,
    pub total_payment: String,
}

/// Position information
#[derive(Debug, Clone, Deserialize)]
pub struct Position {
    pub owner: String,
    pub market_id: String,
    #[serde(default)]
    pub market_name: Option<String>,
    pub base_position: String,
    pub average_entry_price: String,
    pub mark_price: String,
    pub realized_pnl: String,
    pub unrealized_pnl: String,
    #[serde(default)]
    pub cumulative_funding: Option<String>,
}

/// Open order
#[derive(Debug, Clone, Deserialize)]
pub struct OpenOrder {
    pub order_id: u64,
    pub market_id: String,
    #[serde(default)]
    pub market_name: Option<String>,
    pub owner: String,
    pub side: String,
    pub price: u64,
    pub quantity: u64,
    pub expiry: u64,
    #[serde(default)]
    pub timestamp: Option<u64>,
}

/// Account summary with margin metrics
#[derive(Debug, Clone, Deserialize)]
pub struct AccountSummary {
    #[serde(default)]
    pub owner: Option<String>,
    pub usdc_collateral: f64,
    #[serde(default)]
    pub equity_snapshot: Option<f64>,
    #[serde(default)]
    pub realized_pnl_snapshot: Option<f64>,
    #[serde(default)]
    pub unrealized_pnl_snapshot: Option<f64>,
    #[serde(default)]
    pub initial_margin_snapshot: Option<f64>,
    #[serde(default)]
    pub maintenance_margin_snapshot: Option<f64>,
    #[serde(default)]
    pub free_collateral_snapshot: Option<f64>,
    #[serde(default)]
    pub available_withdrawal_snapshot: Option<f64>,
}

/// Token balances
#[derive(Debug, Clone, Deserialize)]
pub struct Balances {
    #[serde(flatten)]
    pub tokens: std::collections::HashMap<String, TokenBalance>,
}

/// Balance for a single token
#[derive(Debug, Clone, Deserialize)]
pub struct TokenBalance {
    pub available: String,
    pub reserved: String,
}

// =============================================================================
// Default token mints
// =============================================================================

pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

/// Testnet token mints
pub const TESTNET_SOL: &str = "11111111111111111111111111111112";
pub const TESTNET_USDC: &str = "11111111111111111111111111111113";
