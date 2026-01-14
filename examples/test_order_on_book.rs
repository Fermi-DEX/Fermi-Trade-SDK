//! Test that orders actually appear on the orderbook.

use fermi_trade_sdk::{
    ClientConfig, FermiClient, MarginMode, PerpOrder, PositionEffect, Side, TradingKeypair,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("=== Testing Order Visibility on Orderbook ===\n");

    // Generate a new keypair for testing
    let keypair = TradingKeypair::generate();
    let pubkey = keypair.pubkey_string();
    println!("Trading account: {}\n", pubkey);

    // Initialize client
    let config = ClientConfig::default();
    let mut client = FermiClient::new(keypair, config).await?;

    // Step 1: Airdrop USDC
    println!("1. Requesting airdrop of 5000 USDC...");
    client.airdrop(5000.0).await?;
    println!("   Airdrop requested. Waiting 2 seconds for processing...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Step 2: Check balance
    println!("\n2. Checking account balance...");
    let account = client.get_account().await?;
    println!("   USDC collateral: {}", account.usdc_collateral);

    // Step 3: Get market
    let markets = client.get_markets().await?;
    let market = markets.iter().find(|m| m.name == "SOL-PERP")
        .expect("SOL-PERP market not found");
    println!("\n3. Using market: {} ({})", market.name, market.uuid);

    // Step 4: Check current orderbook
    println!("\n4. Current orderbook state...");
    let book_before = client.get_orderbook(&market.uuid).await?;
    println!("   Bids: {}, Asks: {}", book_before.buys.len(), book_before.sells.len());

    // Step 5: Place a SELL order at a high price (unlikely to be filled)
    // Using a distinctive price we can search for
    let test_price = 250.00;  // Above current market
    let test_qty = 0.5;

    println!("\n5. Placing SELL order: {} SOL @ ${} (5x leverage)...", test_qty, test_price);
    let order = PerpOrder {
        side: Side::Sell,
        price: test_price,
        quantity: test_qty,
        leverage: 5,
        position_effect: PositionEffect::Open,
        margin_mode: MarginMode::Cross,
        reduce_only: false,
    };

    let result = client.place_perp_order(&market.uuid, order).await?;
    println!("   Order placed!");
    println!("   Order ID: {}", result.order_id);
    println!("   TX hash: {}", result.tx_hash);
    println!("   Sequence: {}", result.sequence_number);

    // Step 6: Wait for order to appear on orderbook
    println!("\n6. Waiting 3 seconds for order to appear on orderbook...");
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Step 7: Check orderbook for our order
    println!("\n7. Checking orderbook for our order...");
    let book_after = client.get_orderbook(&market.uuid).await?;
    println!("   Bids: {}, Asks: {}", book_after.buys.len(), book_after.sells.len());

    // Our order should be in the sells (asks) at price 250 USDC = 250_000_000 micro-USDC
    let expected_price = (test_price * 1_000_000.0) as u64;  // 250_000_000
    let expected_qty = (test_qty * 1_000_000_000.0) as u64;  // 500_000_000 (9 decimals for SOL)

    println!("   Looking for order: price={}, qty={}, owner={}", expected_price, expected_qty, pubkey);

    let our_order = book_after.sells.iter().find(|o| o.owner == pubkey);

    if let Some(order) = our_order {
        println!("\n   ✓ ORDER FOUND ON ORDERBOOK!");
        println!("   Order ID: {}", order.order_id);
        println!("   Price: {} (expected {})", order.price, expected_price);
        println!("   Quantity: {} (expected {})", order.quantity, expected_qty);
        println!("   Owner: {}", order.owner);
    } else {
        println!("\n   ✗ Order not found in orderbook sells.");
        println!("   Checking if it might have been filled or if there's a timing issue...");

        // List first few asks to debug
        println!("\n   First 5 asks:");
        for (i, ask) in book_after.sells.iter().take(5).enumerate() {
            println!("   {}. price={} qty={} owner={}", i+1, ask.price, ask.quantity, ask.owner);
        }
    }

    // Step 8: Check our open orders via API
    println!("\n8. Checking open orders via API...");
    let my_orders = client.get_my_orders().await?;
    println!("   Open orders: {}", my_orders.len());
    for ord in &my_orders {
        println!("   - {} {} @ {} qty={}", ord.side, ord.market_id, ord.price, ord.quantity);
    }

    // Step 9: Cancel the order
    println!("\n9. Cancelling order {}...", result.order_id);
    let cancel_result = client.cancel_order(&market.uuid, result.order_id).await?;
    println!("   Cancelled! TX hash: {}", cancel_result.tx_hash);

    // Step 10: Verify order is gone
    println!("\n10. Waiting 2 seconds and verifying order is removed...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    let book_final = client.get_orderbook(&market.uuid).await?;
    let order_still_there = book_final.sells.iter().any(|o| o.owner == pubkey);

    if order_still_there {
        println!("   ✗ Order still on book (may need more time to process cancel)");
    } else {
        println!("   ✓ Order successfully removed from orderbook!");
    }

    println!("\n=== Test Complete ===");
    Ok(())
}
