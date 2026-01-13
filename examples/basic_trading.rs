//! Basic trading example demonstrating the Fermi Trade SDK.
//!
//! This example shows how to:
//! 1. Initialize the client with a keypair
//! 2. Airdrop testnet USDC
//! 3. Query markets and orderbook
//! 4. Place a perpetual order
//! 5. Check positions and account
//! 6. Cancel the order

use fermi_trade_sdk::{
    ClientConfig, FermiClient, MarginMode, PerpOrder, PositionEffect, Side, TradingKeypair,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Fermi Trade SDK Example ===\n");

    // Load keypair - you can use any of these methods:
    // 1. From file (64-byte JSON array)
    // let keypair = TradingKeypair::from_file("./my_keypair.json")?;

    // 2. Generate a new random keypair (for testing)
    let keypair = TradingKeypair::generate();

    println!("Trading account: {}\n", keypair.pubkey_string());

    // Configure endpoints (uses env vars or defaults to localhost)
    // Set FERMI_CONTINUUM_ENDPOINT and FERMI_RPC_ENDPOINT env vars to override
    let config = ClientConfig::default();

    // Initialize client
    let mut client = FermiClient::new(keypair, config).await?;

    // === Step 1: Airdrop testnet USDC ===
    println!("1. Requesting airdrop of 1000 USDC...");
    match client.airdrop(1000.0).await {
        Ok(_) => println!("   Airdrop successful!\n"),
        Err(e) => println!("   Airdrop failed (may need testnet): {}\n", e),
    }

    // === Step 2: Check account balance ===
    println!("2. Checking account balance...");
    let account = client.get_account().await?;
    println!("   USDC collateral: {}\n", account.usdc_collateral);

    // === Step 3: Get available markets ===
    println!("3. Fetching available markets...");
    let markets = client.get_markets().await?;
    println!("   Found {} markets:", markets.len());
    for market in &markets {
        println!("   - {} ({}): {}", market.name, market.kind, market.uuid);
    }
    println!();

    // Find a perp market (e.g., SOL-PERP)
    let perp_market = markets.iter().find(|m| m.kind == "perp" || m.name.contains("PERP"));

    if let Some(market) = perp_market {
        println!("4. Using market: {} ({})\n", market.name, market.uuid);

        // === Step 4: Get orderbook ===
        println!("5. Fetching orderbook...");
        match client.get_orderbook(&market.uuid).await {
            Ok(orderbook) => {
                println!("   Bids: {}, Asks: {}", orderbook.buys.len(), orderbook.sells.len());
                if let Some(best_bid) = orderbook.buys.first() {
                    println!("   Best bid: {} @ {}", best_bid.quantity, best_bid.price);
                }
                if let Some(best_ask) = orderbook.sells.first() {
                    println!("   Best ask: {} @ {}", best_ask.quantity, best_ask.price);
                }
            }
            Err(e) => println!("   Failed to fetch orderbook: {}", e),
        }
        println!();

        // === Step 5: Place a perp order ===
        println!("6. Placing a 10x long order...");
        let order = PerpOrder {
            side: Side::Buy,
            price: 185.50,    // Price in USDC
            quantity: 0.1,    // Quantity in base asset (e.g., SOL)
            leverage: 10,
            position_effect: PositionEffect::Open,
            margin_mode: MarginMode::Cross,
            reduce_only: false,
        };

        match client.place_perp_order(&market.uuid, order).await {
            Ok(result) => {
                println!("   Order placed successfully!");
                println!("   Order ID: {}", result.order_id);
                println!("   Sequence: {}", result.sequence_number);
                println!("   Expected tick: {}", result.expected_tick);
                println!("   TX hash: {}", result.tx_hash);

                // === Step 6: Check positions ===
                println!("\n7. Checking positions...");
                let positions = client.get_positions().await?;
                println!("   Open positions: {}", positions.len());
                for pos in &positions {
                    println!(
                        "   - {} {}: entry={}, mark={}",
                        pos.market_id,
                        pos.base_position,
                        pos.average_entry_price,
                        pos.mark_price
                    );
                }

                // === Step 7: Check open orders ===
                println!("\n8. Checking open orders...");
                let orders = client.get_my_orders().await?;
                println!("   Open orders: {}", orders.len());
                for ord in &orders {
                    println!(
                        "   - {} {} {} @ {} qty={}",
                        ord.order_id, ord.side, ord.market_id, ord.price, ord.quantity
                    );
                }

                // === Step 8: Cancel the order ===
                println!("\n9. Cancelling order {}...", result.order_id);
                match client.cancel_order(&market.uuid, result.order_id).await {
                    Ok(cancel_result) => {
                        println!("   Order cancelled successfully!");
                        println!("   TX hash: {}", cancel_result.tx_hash);
                    }
                    Err(e) => println!("   Cancel failed: {}", e),
                }
            }
            Err(e) => println!("   Order placement failed: {}", e),
        }
    } else {
        println!("No perp market found. Make sure the node has perp markets configured.");
    }

    println!("\n=== Example Complete ===");

    Ok(())
}
