//! Debug airdrop and account state

use fermi_trade_sdk::{
    ClientConfig, FermiClient, TradingKeypair, TESTNET_USDC,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("=== Debug Airdrop and Account State ===\n");

    let keypair = TradingKeypair::generate();
    let pubkey = keypair.pubkey_string();
    println!("Trading account: {}\n", pubkey);

    let config = ClientConfig::default();
    let mut client = FermiClient::new(keypair, config).await?;

    // Check initial state
    println!("1. Initial account state:");
    let account = client.get_account().await?;
    println!("   Account: {:?}", account);

    println!("\n2. Initial balances:");
    let balances = client.get_balances().await?;
    println!("   Balances: {:?}", balances);

    // Airdrop
    println!("\n3. Requesting airdrop of 10000 USDC...");
    match client.airdrop(10000.0).await {
        Ok(_) => println!("   Airdrop request succeeded"),
        Err(e) => println!("   Airdrop error: {:?}", e),
    }

    // Wait longer
    println!("\n4. Waiting 5 seconds...");
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Check again
    println!("\n5. Account state after airdrop:");
    let account = client.get_account().await?;
    println!("   Account: {:?}", account);

    println!("\n6. Balances after airdrop:");
    let balances = client.get_balances().await?;
    println!("   Balances: {:?}", balances);

    // Try direct airdrop_to with explicit mint
    println!("\n7. Trying direct airdrop_to with TESTNET_USDC mint...");
    println!("   TESTNET_USDC = {}", TESTNET_USDC);
    let amount_micro = 5000_000_000u64; // 5000 USDC in micro units
    match client.airdrop_to(&pubkey, TESTNET_USDC, amount_micro).await {
        Ok(_) => println!("   airdrop_to succeeded"),
        Err(e) => println!("   airdrop_to error: {:?}", e),
    }

    println!("\n8. Waiting 5 more seconds...");
    tokio::time::sleep(Duration::from_secs(5)).await;

    println!("\n9. Final account state:");
    let account = client.get_account().await?;
    println!("   Account: {:?}", account);

    println!("\n10. Final balances:");
    let balances = client.get_balances().await?;
    println!("   Balances: {:?}", balances);

    println!("\n=== Debug Complete ===");
    Ok(())
}
