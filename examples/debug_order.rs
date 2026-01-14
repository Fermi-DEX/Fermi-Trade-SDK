//! Debug order signing and submission

use fermi_trade_sdk::{
    ClientConfig, FermiClient, MarginMode, PerpOrder, PositionEffect, Side, TradingKeypair,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Set RUST_LOG=debug for more details
    tracing_subscriber::fmt::init();

    println!("=== Debug Order Signing ===\n");

    let keypair = TradingKeypair::generate();
    let pubkey = keypair.pubkey_string();
    println!("Trading account: {}\n", pubkey);

    let config = ClientConfig::default();
    let mut client = FermiClient::new(keypair, config).await?;

    // Airdrop and wait
    println!("1. Airdrop and wait...");
    client.airdrop(10000.0).await?;

    for i in 1..=20 {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let acc = client.get_account().await?;
        if acc.usdc_collateral >= 1000.0 {
            println!("   Airdrop confirmed after {} seconds: {} USDC", i, acc.usdc_collateral);
            break;
        }
        if i == 20 {
            println!("   Airdrop not confirmed after 20 seconds");
            return Ok(());
        }
    }

    // Get market info
    println!("\n2. Market info...");
    let markets = client.get_markets().await?;
    let market = markets.iter().find(|m| m.name == "SOL-PERP").unwrap();
    println!("   UUID: {}", market.uuid);
    println!("   Base mint: {}", market.base_mint);
    println!("   Quote mint: {}", market.quote_mint);
    println!("   Base decimals: {}", market.base_decimals);
    println!("   Quote decimals: {}", market.quote_decimals);

    // Place order with debug
    println!("\n3. Placing order...");
    let order = PerpOrder {
        side: Side::Sell,
        price: 200.0,
        quantity: 1.0,
        leverage: 5,
        position_effect: PositionEffect::Open,
        margin_mode: MarginMode::Cross,
        reduce_only: false,
    };

    // Calculate what the canonical values should be
    let price_canonical = (200.0 * 10f64.powi(market.quote_decimals as i32)) as u64;
    let qty_canonical = (1.0 * 10f64.powi(market.base_decimals as i32)) as u64;
    println!("   Expected price canonical: {} (200.0 * 10^{})", price_canonical, market.quote_decimals);
    println!("   Expected qty canonical: {} (1.0 * 10^{})", qty_canonical, market.base_decimals);

    let result = client.place_perp_order(&market.uuid, order).await?;
    println!("   Order ID: {}", result.order_id);
    println!("   TX hash: {}", result.tx_hash);
    println!("   Sequence: {}", result.sequence_number);

    // Wait and check
    println!("\n4. Waiting 10 seconds...");
    tokio::time::sleep(Duration::from_secs(10)).await;

    println!("\n5. Checking orderbook for our order...");
    let book = client.get_orderbook(&market.uuid).await?;

    // Look at asks around our price
    println!("   Looking for orders from {}", pubkey);
    println!("   Total asks: {}", book.sells.len());

    let mut found = false;
    for ask in &book.sells {
        if ask.owner == pubkey {
            println!("   FOUND: price={} qty={} owner={}", ask.price, ask.quantity, ask.owner);
            found = true;
        }
    }

    if !found {
        println!("   NOT FOUND on orderbook");

        // Check open orders
        println!("\n6. Checking open orders API...");
        let orders = client.get_my_orders().await?;
        println!("   Found {} open orders", orders.len());
        for o in &orders {
            println!("   - ID:{} {} @ {} qty={}", o.order_id, o.side, o.price, o.quantity);
        }

        // Show some asks to compare
        println!("\n7. Sample asks from orderbook:");
        for (i, ask) in book.sells.iter().take(5).enumerate() {
            println!("   {}: price={} qty={} owner={}...", i, ask.price, ask.quantity, &ask.owner[..10]);
        }
    }

    // Cancel
    println!("\n8. Cancelling order...");
    let _ = client.cancel_order(&market.uuid, result.order_id).await;

    println!("\n=== Done ===");
    Ok(())
}
