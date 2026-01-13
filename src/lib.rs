//! # Fermi Trade SDK
//!
//! A standalone Rust SDK for the Fermi perpetuals DEX, focused on the Continuum route.
//!
//! ## Features
//!
//! - Place and cancel perpetual orders via Continuum
//! - Query market data, orderbooks, and positions
//! - Multi-format keypair support (file, bytes, base58)
//! - Human-readable price/quantity inputs with automatic decimal conversion
//! - Testnet airdrop functionality
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use fermi_trade_sdk::{FermiClient, TradingKeypair, PerpOrder, Side, PositionEffect, MarginMode, ClientConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Load keypair from file
//!     let keypair = TradingKeypair::from_file("./my_keypair.json")?;
//!
//!     // Initialize client (uses FERMI_CONTINUUM_ENDPOINT and FERMI_RPC_ENDPOINT env vars, or defaults)
//!     let mut client = FermiClient::new(keypair, ClientConfig::default()).await?;
//!
//!     // Airdrop testnet USDC
//!     client.airdrop(1000.0).await?;
//!
//!     // Get markets
//!     let markets = client.get_markets().await?;
//!     let sol_perp = markets.iter().find(|m| m.name == "SOL-PERP").unwrap();
//!
//!     // Place a long position
//!     let order = PerpOrder {
//!         side: Side::Buy,
//!         price: 185.50,
//!         quantity: 1.0,
//!         leverage: 10,
//!         position_effect: PositionEffect::Open,
//!         margin_mode: MarginMode::Cross,
//!         reduce_only: false,
//!     };
//!
//!     let result = client.place_perp_order(&sol_perp.uuid, order).await?;
//!     println!("Order placed: {}", result.tx_hash);
//!
//!     Ok(())
//! }
//! ```

// Internal modules
mod client;
mod continuum;
mod error;
mod keypair;
mod rpc;
mod signing;
mod types;

// Re-export public API
pub use client::{ClientConfig, FermiClient};
pub use error::{Result, SdkError};
pub use keypair::TradingKeypair;
pub use types::{
    // Enums
    MarginMode,
    PositionEffect,
    Side,
    // Order types
    CancelResult,
    OrderResult,
    PerpOrder,
    // Market types
    Depth,
    FundingEvent,
    MarketInfo,
    OpenOrder,
    Orderbook,
    OrderbookEntry,
    Trade,
    // Account types
    AccountSummary,
    Balances,
    Position,
    TokenBalance,
    // Pubkey
    Pubkey,
    // Constants
    SOL_MINT,
    TESTNET_SOL,
    TESTNET_USDC,
    USDC_MINT,
};

// Re-export Continuum status for advanced users
pub use continuum::SequencerStatus;
