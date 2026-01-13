# Fermi Trade SDK

A standalone Rust SDK for trading perpetual futures on the Fermi DEX via the Continuum sequencer.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
fermi-trade-sdk = { path = "../fermi-trade-sdk" }
tokio = { version = "1", features = ["full"] }
anyhow = "1.0"
```

## Quick Start

```rust
use fermi_trade_sdk::{
    FermiClient, TradingKeypair, PerpOrder, Side,
    PositionEffect, MarginMode, ClientConfig
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Load your keypair
    let keypair = TradingKeypair::from_file("./keypair.json")?;

    // 2. Connect to Fermi (uses env vars or defaults to localhost)
    let mut client = FermiClient::new(keypair, ClientConfig::default()).await?;

    // 3. Fund your account (testnet only)
    client.airdrop(1000.0).await?;

    // 4. Find a market
    let markets = client.get_markets().await?;
    let market = markets.iter().find(|m| m.name == "SOL-PERP").unwrap();

    // 5. Place an order
    let order = PerpOrder {
        side: Side::Buy,
        price: 185.50,        // Human-readable USDC price
        quantity: 1.0,        // Human-readable SOL quantity
        leverage: 10,
        position_effect: PositionEffect::Open,
        margin_mode: MarginMode::Cross,
        reduce_only: false,
    };

    let result = client.place_perp_order(&market.uuid, order).await?;
    println!("Order placed: {}", result.tx_hash);

    // 6. Cancel if needed
    client.cancel_order(&market.uuid, result.order_id).await?;

    Ok(())
}
```

## Keypair Formats

The SDK supports multiple keypair formats:

```rust
// From JSON file (64-byte array: [secret_32_bytes, public_32_bytes])
let keypair = TradingKeypair::from_file("./keypair.json")?;

// From raw bytes
let keypair = TradingKeypair::from_bytes(&[u8; 64])?;

// From base58 secret key (32 bytes, public key derived)
let keypair = TradingKeypair::from_base58_secret("your_base58_secret")?;

// Generate new random keypair (for testing)
let keypair = TradingKeypair::generate();

// Get public key
println!("Account: {}", keypair.pubkey_string());
```

### Creating a Keypair File

```bash
# Using solana-keygen (if available)
solana-keygen new -o ./keypair.json

# Or create manually - JSON array of 64 bytes
echo '[1,2,3,...64 bytes total...]' > keypair.json
```

## API Reference

### Trading Operations

```rust
// Place a perpetual order
let result = client.place_perp_order(&market_id, PerpOrder {
    side: Side::Buy,              // or Side::Sell
    price: 185.50,                // Price in quote currency (USDC)
    quantity: 1.0,                // Quantity in base currency (SOL)
    leverage: 10,                 // 1-100x
    position_effect: PositionEffect::Open,   // or Close
    margin_mode: MarginMode::Cross,          // or Isolated
    reduce_only: false,
}).await?;

// Cancel an order
client.cancel_order(&market_id, order_id).await?;
```

### Read Operations

```rust
// Markets
let markets = client.get_markets().await?;
let market = client.get_market(&market_id).await?;
let orderbook = client.get_orderbook(&market_id).await?;
let depth = client.get_depth(&market_id).await?;  // Binance-style
let trades = client.get_trades(&market_id).await?;
let funding = client.get_funding(&market_id).await?;

// Account
let account = client.get_account().await?;        // Margin metrics
let balances = client.get_balances().await?;      // Token balances
let positions = client.get_positions().await?;    // Open positions
let orders = client.get_my_orders().await?;       // Open orders
```

### Testnet Funding

```rust
// Airdrop USDC to your account (testnet only)
client.airdrop(1000.0).await?;  // 1000 USDC

// Airdrop to another address
client.airdrop_to(&recipient_pubkey, TESTNET_USDC, amount_micro).await?;
```

## Configuration

The SDK uses environment variables for endpoint configuration, with localhost defaults:

```bash
# Set these environment variables (optional)
export FERMI_CONTINUUM_ENDPOINT="http://your-continuum:9090"
export FERMI_RPC_ENDPOINT="http://your-rpc:8080"
```

```rust
use fermi_trade_sdk::ClientConfig;

// Default configuration (reads from env vars, falls back to localhost)
let config = ClientConfig::default();
// continuum_endpoint: FERMI_CONTINUUM_ENDPOINT or "http://localhost:9090"
// rpc_endpoint: FERMI_RPC_ENDPOINT or "http://localhost:8080"

// Custom configuration (override programmatically)
let config = ClientConfig {
    continuum_endpoint: "http://your-continuum:9090".into(),
    rpc_endpoint: "http://your-rpc:8080".into(),
};
```

## Order Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `side` | `Side::Buy` / `Side::Sell` | Order direction |
| `price` | `f64` | Price in quote currency (e.g., USDC) |
| `quantity` | `f64` | Amount in base currency (e.g., SOL) |
| `leverage` | `u64` | Leverage multiplier (1-100) |
| `position_effect` | `PositionEffect::Open` / `Close` | Open new or close existing |
| `margin_mode` | `MarginMode::Cross` / `Isolated` | Margin type |
| `reduce_only` | `bool` | Only reduce position, don't increase |

## Response Types

### OrderResult
```rust
pub struct OrderResult {
    pub order_id: u64,        // Unique order ID
    pub sequence_number: u64, // Continuum sequence
    pub expected_tick: u64,   // Expected inclusion tick
    pub tx_hash: String,      // Transaction hash
}
```

### Position
```rust
pub struct Position {
    pub owner: String,
    pub market_id: String,
    pub market_name: Option<String>,
    pub base_position: String,        // Signed position size
    pub average_entry_price: String,
    pub mark_price: String,
    pub realized_pnl: String,
    pub unrealized_pnl: String,
}
```

### AccountSummary
```rust
pub struct AccountSummary {
    pub usdc_collateral: f64,
    pub equity_snapshot: Option<f64>,
    pub realized_pnl_snapshot: Option<f64>,
    pub unrealized_pnl_snapshot: Option<f64>,
    pub initial_margin_snapshot: Option<f64>,
    pub maintenance_margin_snapshot: Option<f64>,
    pub free_collateral_snapshot: Option<f64>,
    pub available_withdrawal_snapshot: Option<f64>,
}
```

## Error Handling

```rust
use fermi_trade_sdk::{Result, SdkError};

match client.place_perp_order(&market_id, order).await {
    Ok(result) => println!("Success: {}", result.tx_hash),
    Err(SdkError::MarketNotFound(id)) => println!("Market {} not found", id),
    Err(SdkError::ContinuumSubmission(msg)) => println!("Submission failed: {}", msg),
    Err(e) => println!("Error: {}", e),
}
```

## Constants

```rust
use fermi_trade_sdk::{SOL_MINT, USDC_MINT, TESTNET_SOL, TESTNET_USDC};

// Mainnet mints
SOL_MINT  = "So11111111111111111111111111111111111111112"
USDC_MINT = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"

// Testnet mints
TESTNET_SOL  = "11111111111111111111111111111112"
TESTNET_USDC = "11111111111111111111111111111113"
```

## Example: Market Making Bot

```rust
use fermi_trade_sdk::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let keypair = TradingKeypair::from_file("./keypair.json")?;
    let mut client = FermiClient::new(keypair, ClientConfig::default()).await?;

    let markets = client.get_markets().await?;
    let market = markets.iter().find(|m| m.name == "SOL-PERP").unwrap();

    loop {
        // Get current orderbook
        let book = client.get_orderbook(&market.uuid).await?;

        let mid_price = if let (Some(bid), Some(ask)) = (book.buys.first(), book.sells.first()) {
            (bid.price + ask.price) as f64 / 2.0 / 1_000_000.0  // Convert from micro-USDC
        } else {
            185.0  // Default
        };

        // Place orders around mid price
        let spread = 0.10;  // $0.10 spread

        let buy_order = PerpOrder {
            side: Side::Buy,
            price: mid_price - spread,
            quantity: 0.1,
            leverage: 5,
            position_effect: PositionEffect::Open,
            margin_mode: MarginMode::Cross,
            reduce_only: false,
        };

        let sell_order = PerpOrder {
            side: Side::Sell,
            price: mid_price + spread,
            quantity: 0.1,
            leverage: 5,
            position_effect: PositionEffect::Open,
            margin_mode: MarginMode::Cross,
            reduce_only: false,
        };

        let buy_result = client.place_perp_order(&market.uuid, buy_order).await?;
        let sell_result = client.place_perp_order(&market.uuid, sell_order).await?;

        println!("Placed orders at {:.2}/{:.2}", mid_price - spread, mid_price + spread);

        // Wait and cancel
        tokio::time::sleep(Duration::from_secs(5)).await;

        let _ = client.cancel_order(&market.uuid, buy_result.order_id).await;
        let _ = client.cancel_order(&market.uuid, sell_result.order_id).await;
    }
}
```

## Running the Example

```bash
cd fermi-trade-sdk
cargo run --example basic_trading
```

## Architecture

```
User Code
    │
    ▼
┌─────────────────┐
│  FermiClient    │  ◄── Main facade
├─────────────────┤
│ - keypair       │
│ - continuum     │  ◄── gRPC client (orders/cancels)
│ - rpc           │  ◄── HTTP client (reads)
└─────────────────┘
         │
         ├──────────────────┬────────────────────┐
         ▼                  ▼                    ▼
┌─────────────────┐  ┌─────────────┐  ┌──────────────────┐
│ ContinuumClient │  │  RpcClient  │  │ signing module   │
│ (gRPC)          │  │  (HTTP)     │  │ (ed25519+borsh)  │
└─────────────────┘  └─────────────┘  └──────────────────┘
         │                  │
         ▼                  ▼
   Continuum           Rollup Node
   Sequencer           RPC Server
```

## License

MIT
