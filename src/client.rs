//! Main FermiClient facade for the SDK.
//!
//! Provides a unified interface for all trading operations.

use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;

use crate::continuum::ContinuumClient;
use crate::error::{Result, SdkError};
use crate::keypair::TradingKeypair;
use crate::rpc::RpcClient;
use crate::signing::{sign_cancel, sign_perp_order};
use crate::types::{
    AccountSummary, Balances, CancelResult, Depth, FundingEvent, MarketInfo, OpenOrder,
    Orderbook, OrderResult, PerpOrder, Position, Pubkey, Trade, TESTNET_USDC,
};

/// Configuration for the Fermi client
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Continuum gRPC endpoint (e.g., "http://localhost:9090")
    pub continuum_endpoint: String,
    /// RPC HTTP endpoint (e.g., "http://localhost:8080")
    pub rpc_endpoint: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            continuum_endpoint: std::env::var("FERMI_CONTINUUM_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:9090".to_string()),
            rpc_endpoint: std::env::var("FERMI_RPC_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
        }
    }
}

/// The main Fermi Trade SDK client.
///
/// Provides methods for:
/// - Placing and cancelling perpetual orders via Continuum
/// - Querying market data, positions, and account information
/// - Testnet airdrop functionality
pub struct FermiClient {
    keypair: TradingKeypair,
    continuum: ContinuumClient,
    rpc: RpcClient,
    #[allow(dead_code)]
    config: ClientConfig,
}

impl FermiClient {
    /// Create a new FermiClient with the given keypair and configuration.
    pub async fn new(keypair: TradingKeypair, config: ClientConfig) -> Result<Self> {
        let continuum = ContinuumClient::connect(&config.continuum_endpoint).await?;
        let rpc = RpcClient::new(&config.rpc_endpoint);

        info!(
            "FermiClient initialized for account: {}",
            keypair.pubkey_string()
        );

        Ok(Self {
            keypair,
            continuum,
            rpc,
            config,
        })
    }

    /// Get the public key of the trading account as a string.
    pub fn pubkey(&self) -> String {
        self.keypair.pubkey_string()
    }

    /// Get the public key as a Pubkey type.
    pub fn pubkey_bytes(&self) -> Pubkey {
        self.keypair.pubkey()
    }

    // =========================================================================
    // Trading operations (via Continuum)
    // =========================================================================

    /// Place a perpetual order.
    ///
    /// This method:
    /// 1. Fetches market decimals to convert price/quantity to canonical units
    /// 2. Calculates margin amount if not specified
    /// 3. Signs the order
    /// 4. Submits to Continuum
    pub async fn place_perp_order(
        &mut self,
        market_id: &str,
        order: PerpOrder,
    ) -> Result<OrderResult> {
        // Fetch market info for decimal conversion
        let market = self.rpc.get_market(market_id).await?;

        // Convert human-readable price/quantity to canonical units
        let (price_canonical, qty_canonical) =
            self.to_canonical(&market, order.price, order.quantity)?;

        // Calculate margin amount if not provided
        let margin_amount = self.calculate_margin(order.price, order.quantity, order.leverage);

        // Parse mints
        let base_mint = Pubkey::from_str(&market.base_mint)
            .map_err(|e| SdkError::InvalidPubkey(format!("base_mint: {}", e)))?;
        let quote_mint = Pubkey::from_str(&market.quote_mint)
            .map_err(|e| SdkError::InvalidPubkey(format!("quote_mint: {}", e)))?;

        // Generate order ID
        let order_id = generate_order_id();

        // Calculate expiry (1 hour from now)
        let expiry = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| SdkError::Signing(e.to_string()))?
            .as_secs()
            + 3600;

        // Sign the order
        let signed_order = sign_perp_order(
            &self.keypair,
            order_id,
            order.side,
            price_canonical,
            qty_canonical,
            expiry,
            &base_mint,
            &quote_mint,
            order.leverage,
            order.position_effect,
            order.margin_mode,
            Some(margin_amount),
            order.reduce_only,
        )?;

        info!(
            "Placing {} perp order: price={}, qty={}, leverage={}x",
            order.side, order.price, order.quantity, order.leverage
        );

        // Submit to Continuum
        let result = self.continuum.submit_order(&signed_order).await?;

        info!(
            "Order {} placed successfully, tx_hash: {}",
            result.order_id, result.tx_hash
        );

        Ok(result)
    }

    /// Cancel an existing order.
    pub async fn cancel_order(&mut self, market_id: &str, order_id: u64) -> Result<CancelResult> {
        // Fetch market info for mints
        let market = self.rpc.get_market(market_id).await?;

        let base_mint = Pubkey::from_str(&market.base_mint)
            .map_err(|e| SdkError::InvalidPubkey(format!("base_mint: {}", e)))?;
        let quote_mint = Pubkey::from_str(&market.quote_mint)
            .map_err(|e| SdkError::InvalidPubkey(format!("quote_mint: {}", e)))?;

        // Sign the cancel
        let signed_cancel = sign_cancel(&self.keypair, order_id, &base_mint, &quote_mint)?;

        info!("Cancelling order {}", order_id);

        // Submit to Continuum
        let result = self.continuum.submit_cancel(&signed_cancel).await?;

        info!(
            "Order {} cancelled successfully, tx_hash: {}",
            result.order_id, result.tx_hash
        );

        Ok(result)
    }

    // =========================================================================
    // Testnet funding
    // =========================================================================

    /// Airdrop USDC to your own account (testnet only).
    ///
    /// Amount is in human-readable USDC (e.g., 1000.0 for 1000 USDC).
    pub async fn airdrop(&self, amount: f64) -> Result<()> {
        // Convert to micro-USDC (6 decimals)
        let amount_micro = (amount * 1_000_000.0) as u64;
        self.rpc
            .airdrop(&self.pubkey(), TESTNET_USDC, amount_micro)
            .await
    }

    /// Airdrop tokens to a specific recipient (testnet only).
    pub async fn airdrop_to(&self, recipient: &str, token_mint: &str, amount: u64) -> Result<()> {
        self.rpc.airdrop(recipient, token_mint, amount).await
    }

    // =========================================================================
    // Read operations (via RPC)
    // =========================================================================

    /// Get all available markets.
    pub async fn get_markets(&self) -> Result<Vec<MarketInfo>> {
        self.rpc.list_markets().await
    }

    /// Get a specific market by UUID.
    pub async fn get_market(&self, market_id: &str) -> Result<MarketInfo> {
        self.rpc.get_market(market_id).await
    }

    /// Get the orderbook for a market.
    pub async fn get_orderbook(&self, market_id: &str) -> Result<Orderbook> {
        self.rpc.get_orderbook(market_id).await
    }

    /// Get depth data (Binance-style format).
    pub async fn get_depth(&self, market_id: &str) -> Result<Depth> {
        self.rpc.get_depth(market_id).await
    }

    /// Get recent trades for a market.
    pub async fn get_trades(&self, market_id: &str) -> Result<Vec<Trade>> {
        self.rpc.get_trades(market_id).await
    }

    /// Get funding events for a market.
    pub async fn get_funding(&self, market_id: &str) -> Result<Vec<FundingEvent>> {
        self.rpc.get_funding(market_id).await
    }

    /// Get your positions.
    pub async fn get_positions(&self) -> Result<Vec<Position>> {
        self.rpc.get_positions(Some(&self.pubkey())).await
    }

    /// Get all positions (all users).
    pub async fn get_all_positions(&self) -> Result<Vec<Position>> {
        self.rpc.get_positions(None).await
    }

    /// Get your open orders.
    pub async fn get_my_orders(&self) -> Result<Vec<OpenOrder>> {
        self.rpc.get_user_orders(&self.pubkey()).await
    }

    /// Get your account summary (balances and margin metrics).
    pub async fn get_account(&self) -> Result<AccountSummary> {
        self.rpc.get_account(&self.pubkey()).await
    }

    /// Get your token balances.
    pub async fn get_balances(&self) -> Result<Balances> {
        self.rpc.get_balances(&self.pubkey()).await
    }

    // =========================================================================
    // Helper methods
    // =========================================================================

    /// Convert human-readable price/quantity to canonical units.
    fn to_canonical(&self, market: &MarketInfo, price: f64, quantity: f64) -> Result<(u64, u64)> {
        let quote_multiplier = 10f64.powi(market.quote_decimals as i32);
        let base_multiplier = 10f64.powi(market.base_decimals as i32);

        let price_canonical = (price * quote_multiplier) as u64;
        let qty_canonical = (quantity * base_multiplier) as u64;

        Ok((price_canonical, qty_canonical))
    }

    /// Calculate margin amount based on price, quantity, and leverage.
    /// Returns amount in quote token base units (micro-USDC).
    fn calculate_margin(&self, price: f64, quantity: f64, leverage: u64) -> u64 {
        let notional = price * quantity;
        let margin = notional / (leverage as f64);
        // Convert to micro-USDC (6 decimals)
        (margin * 1_000_000.0) as u64
    }
}

/// Generate a unique order ID based on timestamp.
fn generate_order_id() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64
}
