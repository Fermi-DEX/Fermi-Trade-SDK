//! Full trading test: airdrop -> place order -> verify on orderbook -> cancel

use fermi_trade_sdk::{
    ClientConfig, FermiClient, MarginMode, PerpOrder, PositionEffect, Side, TradingKeypair,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("=== Full Trading Test ===\n");

    let keypair = TradingKeypair::generate();
    let pubkey = keypair.pubkey_string();
    println!("Trading account: {}\n", pubkey);

    let config = ClientConfig::default();
    let mut client = FermiClient::new(keypair, config).await?;

    // === STEP 1: Airdrop and wait for processing ===
    println!("STEP 1: Airdrop 10000 USDC and wait for processing...");
    client.airdrop(10000.0).await?;
    println!("   Airdrop requested. Polling for confirmation (up to 30 seconds)...");

    let mut attempts = 0;
    let max_attempts = 15;
    let mut collateral = 0.0;

    while attempts < max_attempts {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let account = client.get_account().await?;
        collateral = account.usdc_collateral;
        attempts += 1;
        print!("   Attempt {}/{}: USDC collateral = {:.2}", attempts, max_attempts, collateral);

        if collateral >= 1000.0 {
            println!(" ✓");
            break;
        }
        println!();
    }

    if collateral < 1000.0 {
        println!("   ERROR: Airdrop not confirmed after {} attempts. Exiting.", max_attempts);
        return Ok(());
    }
    println!("   ✓ Airdrop confirmed! Collateral: {}\n", collateral);

    // === STEP 2: Get market info ===
    println!("STEP 2: Get SOL-PERP market...");
    let markets = client.get_markets().await?;
    let market = markets.iter().find(|m| m.name == "SOL-PERP")
        .expect("SOL-PERP market not found");
    println!("   Market: {} ({})", market.name, market.uuid);
    println!("   Base decimals: {}, Quote decimals: {}\n", market.base_decimals, market.quote_decimals);

    // === STEP 3: Check orderbook before ===
    println!("STEP 3: Current orderbook state...");
    let book_before = client.get_orderbook(&market.uuid).await?;
    println!("   Bids: {}, Asks: {}", book_before.buys.len(), book_before.sells.len());

    // Show best bid/ask
    if let Some(best_bid) = book_before.buys.first() {
        let bid_price = best_bid.price as f64 / 1_000_000.0;
        println!("   Best bid: ${:.2}", bid_price);
    }
    if let Some(best_ask) = book_before.sells.first() {
        let ask_price = best_ask.price as f64 / 1_000_000.0;
        println!("   Best ask: ${:.2}", ask_price);
    }
    println!();

    // === STEP 4: Place a SELL order above market ===
    // Use a price that's clearly above market to ensure it rests on book
    let test_price = 200.00;  // Well above current ~144
    let test_qty = 1.0;

    println!("STEP 4: Place SELL order: {} SOL @ ${} (5x leverage)...", test_qty, test_price);
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
    println!("   Order submitted!");
    println!("   Order ID: {}", result.order_id);
    println!("   TX hash: {}", result.tx_hash);
    println!("   Sequence: {}", result.sequence_number);
    println!("   Expected tick: {}", result.expected_tick);
    println!();

    // === STEP 5: Wait and check orderbook ===
    println!("STEP 5: Wait 5 seconds for order to appear on orderbook...");
    tokio::time::sleep(Duration::from_secs(5)).await;

    println!("   Checking orderbook...");
    let book_after = client.get_orderbook(&market.uuid).await?;
    println!("   Bids: {}, Asks: {}", book_after.buys.len(), book_after.sells.len());

    // Search for our order
    let our_order = book_after.sells.iter().find(|o| o.owner == pubkey);

    if let Some(order) = our_order {
        println!("\n   ✓ ORDER FOUND ON ORDERBOOK!");
        println!("   Order ID: {}", order.order_id);
        println!("   Price: {} (${:.2})", order.price, order.price as f64 / 1_000_000.0);
        println!("   Quantity: {} ({:.4} SOL)", order.quantity, order.quantity as f64 / 1_000_000_000.0);
        println!("   Owner: {}", order.owner);
    } else {
        println!("\n   ✗ Order not found in orderbook asks.");

        // Check our open orders via API
        println!("\n   Checking open orders via API...");
        let my_orders = client.get_my_orders().await?;
        println!("   Found {} open orders:", my_orders.len());
        for ord in &my_orders {
            println!("   - ID:{} {} {} @ {} qty={}",
                ord.order_id, ord.side, ord.market_id, ord.price, ord.quantity);
        }

        // Show asks around our price range
        println!("\n   Asks near $200 range:");
        for ask in book_after.sells.iter() {
            let price = ask.price as f64 / 1_000_000.0;
            if price > 190.0 && price < 210.0 {
                println!("   - ${:.2} qty={} owner={}", price, ask.quantity, ask.owner);
            }
        }
    }

    // === STEP 6: Check positions ===
    println!("\nSTEP 6: Check positions...");
    let positions = client.get_positions().await?;
    println!("   Open positions: {}", positions.len());
    for pos in &positions {
        println!("   - {}: {} @ entry={}", pos.market_id, pos.base_position, pos.average_entry_price);
    }

    // === STEP 7: Cancel order ===
    println!("\nSTEP 7: Cancel order {}...", result.order_id);
    let cancel_result = client.cancel_order(&market.uuid, result.order_id).await?;
    println!("   Cancel submitted!");
    println!("   TX hash: {}", cancel_result.tx_hash);
    println!("   Sequence: {}", cancel_result.sequence_number);

    // === STEP 8: Verify cancellation ===
    println!("\nSTEP 8: Wait 3 seconds and verify cancellation...");
    tokio::time::sleep(Duration::from_secs(3)).await;

    let book_final = client.get_orderbook(&market.uuid).await?;
    let still_on_book = book_final.sells.iter().any(|o| o.owner == pubkey);

    if still_on_book {
        println!("   ✗ Order still on book");
    } else {
        println!("   ✓ Order successfully removed from orderbook!");
    }

    // Final account state
    println!("\nFinal account state:");
    let final_account = client.get_account().await?;
    println!("   USDC collateral: {}", final_account.usdc_collateral);
    if let Some(free) = final_account.free_collateral_snapshot {
        println!("   Free collateral: {}", free);
    }

    println!("\n=== Test Complete ===");
    Ok(())
}
