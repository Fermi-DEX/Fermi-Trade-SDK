//! REST API client for reading market data, positions, and account information.

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::{Result, SdkError};
use crate::types::{
    AccountSummary, Balances, Depth, FundingEvent, MarketInfo, OpenOrder, Orderbook, Position,
    Trade,
};

/// REST API client for the Fermi rollup node
pub struct RpcClient {
    client: Client,
    base_url: String,
}

impl RpcClient {
    /// Create a new RPC client
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Create an RPC client with a custom reqwest client
    #[allow(dead_code)]
    pub fn with_client(base_url: &str, client: Client) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    // =========================================================================
    // Market queries
    // =========================================================================

    /// List all available markets
    pub async fn list_markets(&self) -> Result<Vec<MarketInfo>> {
        let url = format!("{}/markets", self.base_url);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(SdkError::Rpc(format!(
                "Failed to fetch markets: {}",
                response.status()
            )));
        }

        let markets: Vec<MarketInfo> = response.json().await?;
        Ok(markets)
    }

    /// Get a specific market by UUID
    pub async fn get_market(&self, market_id: &str) -> Result<MarketInfo> {
        let markets = self.list_markets().await?;
        markets
            .into_iter()
            .find(|m| m.uuid == market_id)
            .ok_or_else(|| SdkError::MarketNotFound(market_id.to_string()))
    }

    /// Get the orderbook for a market
    pub async fn get_orderbook(&self, market_id: &str) -> Result<Orderbook> {
        let url = format!("{}/markets/{}/orderbook", self.base_url, market_id);
        let response = self.client.get(&url).send().await?;

        if response.status().is_client_error() {
            return Err(SdkError::MarketNotFound(market_id.to_string()));
        }

        if !response.status().is_success() {
            return Err(SdkError::Rpc(format!(
                "Failed to fetch orderbook: {}",
                response.status()
            )));
        }

        let orderbook: Orderbook = response.json().await?;
        Ok(orderbook)
    }

    /// Get depth data (Binance-style format)
    pub async fn get_depth(&self, market_id: &str) -> Result<Depth> {
        let url = format!("{}/markets/{}/depth", self.base_url, market_id);
        let response = self.client.get(&url).send().await?;

        if response.status().is_client_error() {
            return Err(SdkError::MarketNotFound(market_id.to_string()));
        }

        if !response.status().is_success() {
            return Err(SdkError::Rpc(format!(
                "Failed to fetch depth: {}",
                response.status()
            )));
        }

        let depth: Depth = response.json().await?;
        Ok(depth)
    }

    /// Get recent trades for a market
    pub async fn get_trades(&self, market_id: &str) -> Result<Vec<Trade>> {
        let url = format!("{}/markets/{}/trades", self.base_url, market_id);
        let response = self.client.get(&url).send().await?;

        if response.status().is_client_error() {
            return Err(SdkError::MarketNotFound(market_id.to_string()));
        }

        if !response.status().is_success() {
            return Err(SdkError::Rpc(format!(
                "Failed to fetch trades: {}",
                response.status()
            )));
        }

        let trades: Vec<Trade> = response.json().await?;
        Ok(trades)
    }

    /// Get funding events for a market
    pub async fn get_funding(&self, market_id: &str) -> Result<Vec<FundingEvent>> {
        let url = format!("{}/markets/{}/funding", self.base_url, market_id);
        let response = self.client.get(&url).send().await?;

        if response.status().is_client_error() {
            return Err(SdkError::MarketNotFound(market_id.to_string()));
        }

        if !response.status().is_success() {
            return Err(SdkError::Rpc(format!(
                "Failed to fetch funding: {}",
                response.status()
            )));
        }

        let events: Vec<FundingEvent> = response.json().await?;
        Ok(events)
    }

    // =========================================================================
    // Account queries
    // =========================================================================

    /// Get account summary for an owner
    pub async fn get_account(&self, owner: &str) -> Result<AccountSummary> {
        let url = format!("{}/accounts/{}", self.base_url, owner);
        let response = self.client.get(&url).send().await?;

        if response.status().is_client_error() {
            // Account might not exist yet, return empty account
            return Ok(AccountSummary {
                owner: Some(owner.to_string()),
                usdc_collateral: 0.0,
                equity_snapshot: None,
                realized_pnl_snapshot: None,
                unrealized_pnl_snapshot: None,
                initial_margin_snapshot: None,
                maintenance_margin_snapshot: None,
                free_collateral_snapshot: None,
                available_withdrawal_snapshot: None,
            });
        }

        if !response.status().is_success() {
            return Err(SdkError::Rpc(format!(
                "Failed to fetch account: {}",
                response.status()
            )));
        }

        let account: AccountSummary = response.json().await?;
        Ok(account)
    }

    /// Get token balances for an owner
    pub async fn get_balances(&self, owner: &str) -> Result<Balances> {
        let url = format!("{}/balances/{}", self.base_url, owner);
        let response = self.client.get(&url).send().await?;

        if response.status().is_client_error() {
            // No balances yet
            return Ok(Balances {
                tokens: std::collections::HashMap::new(),
            });
        }

        if !response.status().is_success() {
            return Err(SdkError::Rpc(format!(
                "Failed to fetch balances: {}",
                response.status()
            )));
        }

        let balances: Balances = response.json().await?;
        Ok(balances)
    }

    /// Get positions, optionally filtered by owner
    pub async fn get_positions(&self, owner: Option<&str>) -> Result<Vec<Position>> {
        let url = match owner {
            Some(o) => format!("{}/positions?owner={}", self.base_url, o),
            None => format!("{}/positions", self.base_url),
        };

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(SdkError::Rpc(format!(
                "Failed to fetch positions: {}",
                response.status()
            )));
        }

        let positions: Vec<Position> = response.json().await?;
        Ok(positions)
    }

    /// Get open orders for an owner
    pub async fn get_user_orders(&self, owner: &str) -> Result<Vec<OpenOrder>> {
        let url = format!("{}/orders/user/{}", self.base_url, owner);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(SdkError::Rpc(format!(
                "Failed to fetch user orders: {}",
                response.status()
            )));
        }

        let orders: Vec<OpenOrder> = response.json().await?;
        Ok(orders)
    }

    // =========================================================================
    // Airdrop (testnet only)
    // =========================================================================

    /// Request an airdrop of tokens (testnet only)
    pub async fn airdrop(&self, recipient: &str, token_mint: &str, amount: u64) -> Result<()> {
        #[derive(Serialize)]
        struct AirdropRequest {
            recipient: String,
            token_mint: String,
            amount: u64,
        }

        #[derive(Deserialize)]
        struct AirdropResponse {
            #[allow(dead_code)]
            success: Option<bool>,
            error: Option<String>,
        }

        let url = format!("{}/airdrop", self.base_url);
        let request = AirdropRequest {
            recipient: recipient.to_string(),
            token_mint: token_mint.to_string(),
            amount,
        };

        let response = self.client.post(&url).json(&request).send().await?;

        let status = response.status();
        let body: AirdropResponse = response.json().await.unwrap_or(AirdropResponse {
            success: None,
            error: Some("Failed to parse response".to_string()),
        });

        if !status.is_success() {
            return Err(SdkError::Airdrop(
                body.error.unwrap_or_else(|| format!("HTTP {}", status)),
            ));
        }

        if let Some(err) = body.error {
            return Err(SdkError::Airdrop(err));
        }

        Ok(())
    }

    // =========================================================================
    // Status
    // =========================================================================

    /// Get node status
    #[allow(dead_code)]
    pub async fn get_status(&self) -> Result<NodeStatus> {
        let url = format!("{}/status", self.base_url);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(SdkError::Rpc(format!(
                "Failed to fetch status: {}",
                response.status()
            )));
        }

        let status: NodeStatus = response.json().await?;
        Ok(status)
    }
}

/// Node status information
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct NodeStatus {
    pub block_height: u64,
    pub applied_batches: u64,
}
