//! gRPC client for Continuum sequencer.
//!
//! Handles order and cancel submission via the Continuum ordering service.

use std::time::{SystemTime, UNIX_EPOCH};
use tonic::transport::Channel;
use tracing::{debug, info};

use crate::error::{Result, SdkError};
use crate::signing::{SignedCancel, SignedOrder};
use crate::types::{CancelResult, OrderResult};

// Include the generated protobuf types
pub mod proto {
    tonic::include_proto!("continuum.sequencer.v1");
}

use proto::{
    sequencer_service_client::SequencerServiceClient, GetStatusRequest, SubmitTransactionRequest,
    Transaction,
};

/// Sequencer status information
#[derive(Debug, Clone)]
pub struct SequencerStatus {
    pub current_tick: u64,
    pub total_transactions: u64,
    pub pending_transactions: u64,
    pub uptime_seconds: u64,
    pub transactions_per_second: f64,
}

/// gRPC client for Continuum sequencer
pub struct ContinuumClient {
    client: SequencerServiceClient<Channel>,
    endpoint: String,
}

impl ContinuumClient {
    /// Connect to a Continuum endpoint
    pub async fn connect(endpoint: &str) -> Result<Self> {
        info!("Connecting to Continuum sequencer at: {}", endpoint);

        let channel = Channel::from_shared(endpoint.to_string())
            .map_err(|e| SdkError::ContinuumConnection(format!("Invalid endpoint: {}", e)))?
            .connect()
            .await
            .map_err(|e| SdkError::ContinuumConnection(format!("Connection failed: {}", e)))?;

        let client = SequencerServiceClient::new(channel);

        info!("Successfully connected to Continuum sequencer");

        Ok(Self {
            client,
            endpoint: endpoint.to_string(),
        })
    }

    /// Submit a signed order to Continuum
    pub async fn submit_order(&mut self, signed_order: &SignedOrder) -> Result<OrderResult> {
        let order_json = signed_order.to_json()?;

        // Extract signature from the request
        let signature_bytes = hex::decode(&signed_order.request.signature)
            .map_err(|e| SdkError::Signing(format!("Invalid signature hex: {}", e)))?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| SdkError::Signing(e.to_string()))?
            .as_micros() as u64;

        // Generate transaction ID
        let tx_id = format!("frm_order_{}_{}", signed_order.order_id, timestamp);

        // Build FRM transaction payload
        let mut order_value: serde_json::Value = serde_json::from_str(&order_json)?;
        if let Some(obj) = order_value.as_object_mut() {
            obj.insert(
                "local_sequencer_id".to_string(),
                serde_json::Value::String("fermi_trade_sdk".to_string()),
            );
            obj.entry("type".to_string())
                .or_insert_with(|| serde_json::Value::String("order".to_string()));
            obj.insert(
                "timestamp_ms".to_string(),
                serde_json::Value::String((timestamp / 1000).to_string()),
            );
        }

        let mut frm_fields = serde_json::Map::new();
        frm_fields.insert(
            "version".to_string(),
            serde_json::Value::String("1.0".to_string()),
        );
        if let Some(obj) = order_value.as_object() {
            frm_fields.extend(obj.clone().into_iter());
        }
        let frm_transaction = serde_json::Value::Object(frm_fields);

        let payload_str = format!("FRM_v1.0:{}", frm_transaction);
        debug!("Order FRM payload: {}", payload_str);
        let payload = payload_str.into_bytes();

        let transaction = Transaction {
            tx_id: tx_id.clone(),
            payload,
            signature: signature_bytes,
            public_key: signed_order.owner_bytes.to_vec(),
            nonce: signed_order.order_id,
            timestamp,
        };

        let request = tonic::Request::new(SubmitTransactionRequest {
            transaction: Some(transaction),
        });

        debug!(
            "Submitting order {} to Continuum endpoint {}",
            tx_id, self.endpoint
        );

        let response = self.client.submit_transaction(request).await?.into_inner();

        info!(
            "Order {} submitted successfully, sequence: {}, expected_tick: {}, hash: {}",
            tx_id, response.sequence_number, response.expected_tick, response.tx_hash
        );

        Ok(OrderResult {
            order_id: signed_order.order_id,
            sequence_number: response.sequence_number,
            expected_tick: response.expected_tick,
            tx_hash: response.tx_hash,
        })
    }

    /// Submit a signed cancel to Continuum
    pub async fn submit_cancel(&mut self, signed_cancel: &SignedCancel) -> Result<CancelResult> {
        let cancel_json = signed_cancel.to_json()?;

        // Extract signature from the request
        let signature_bytes = hex::decode(&signed_cancel.request.signature)
            .map_err(|e| SdkError::Signing(format!("Invalid signature hex: {}", e)))?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| SdkError::Signing(e.to_string()))?
            .as_micros() as u64;

        // Generate transaction ID
        let tx_id = format!("frm_cancel_{}_{}", signed_cancel.order_id, timestamp);

        // Build FRM transaction payload
        let mut cancel_value: serde_json::Value = serde_json::from_str(&cancel_json)?;
        if let Some(obj) = cancel_value.as_object_mut() {
            obj.insert(
                "local_sequencer_id".to_string(),
                serde_json::Value::String("fermi_trade_sdk".to_string()),
            );
            obj.entry("type".to_string())
                .or_insert_with(|| serde_json::Value::String("cancel".to_string()));
            obj.insert(
                "timestamp_ms".to_string(),
                serde_json::Value::String((timestamp / 1000).to_string()),
            );
        }

        let mut frm_fields = serde_json::Map::new();
        frm_fields.insert(
            "version".to_string(),
            serde_json::Value::String("1.0".to_string()),
        );
        if let Some(obj) = cancel_value.as_object() {
            frm_fields.extend(obj.clone().into_iter());
        }
        let frm_transaction = serde_json::Value::Object(frm_fields);

        let payload = format!("FRM_v1.0:{}", frm_transaction).into_bytes();

        let transaction = Transaction {
            tx_id: tx_id.clone(),
            payload,
            signature: signature_bytes,
            public_key: signed_cancel.owner_bytes.to_vec(),
            nonce: signed_cancel.order_id,
            timestamp,
        };

        let request = tonic::Request::new(SubmitTransactionRequest {
            transaction: Some(transaction),
        });

        debug!(
            "Submitting cancel {} to Continuum endpoint {}",
            tx_id, self.endpoint
        );

        let response = self.client.submit_transaction(request).await?.into_inner();

        info!(
            "Cancel {} submitted successfully, sequence: {}, expected_tick: {}, hash: {}",
            tx_id, response.sequence_number, response.expected_tick, response.tx_hash
        );

        Ok(CancelResult {
            order_id: signed_cancel.order_id,
            sequence_number: response.sequence_number,
            expected_tick: response.expected_tick,
            tx_hash: response.tx_hash,
        })
    }

    /// Get the current sequencer status
    #[allow(dead_code)]
    pub async fn get_status(&mut self) -> Result<SequencerStatus> {
        let request = tonic::Request::new(GetStatusRequest {});
        let response = self.client.get_status(request).await?.into_inner();

        Ok(SequencerStatus {
            current_tick: response.current_tick,
            total_transactions: response.total_transactions,
            pending_transactions: response.pending_transactions,
            uptime_seconds: response.uptime_seconds,
            transactions_per_second: response.transactions_per_second,
        })
    }
}
