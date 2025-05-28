use std::fs::File;
use std::io::Read;
use std::str::FromStr;

pub use alloy::consensus::SignableTransaction;
pub use alloy::rpc::types::TransactionRequest;
pub use alloy::sol;
pub use alloy::{
    consensus::TxLegacy,
    network::TxSignerSync,
    primitives::{Address, FixedBytes, TxKind, U256},
    providers::{Provider, ProviderBuilder},
    signers::local::PrivateKeySigner,
    sol_types::{SolCall, SolType},
};
use alloy::{hex::FromHex, primitives::TxHash};
pub use rand::Rng;
pub use serde::{Deserialize, Serialize};
pub use std::io::{self, BufRead};
pub use std::path::Path;
use tracing::warn;

pub use anyhow::{Error, Result};
pub use async_trait::async_trait;
pub use dotenv::dotenv;
pub use rayon::prelude::*;
pub use reqwest::Client;
pub use serde_json::Value;
pub use std::time::Duration;
pub use tokio::time::sleep;
pub use tracing::{debug, error, info};

#[derive(Debug, Clone, Copy)]
pub enum DistributionType {
    NativeDirect,
    NativeBatch,
    NFTMint,
    Swapper,
}

impl FromStr for DistributionType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let dist_type = match s.to_lowercase().as_str() {
            "native-direct" | "native_direct" | "nativedirect" | "NativeDirect" => {
                DistributionType::NativeDirect
            }
            "native-batch" | "native_batch" | "nativebatch" | "NativeBatch" => {
                DistributionType::NativeBatch
            }
            "swapper" | "Swapper" => DistributionType::Swapper,
            "nft-mint" | "nft_mint" | "nftmint" | "NFTMint" => DistributionType::NFTMint,
            x => return Err(anyhow::anyhow!("Unknown distribution type: {}", x)),
        };
        Ok(dist_type)
    }
}

impl<'de> Deserialize<'de> for DistributionType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;
        DistributionType::from_str(s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Deserialize)]
pub struct CoreConfig {
    pub rpc_urls: Vec<String>,
    pub target_tps: u64,
    pub rpc_batch_size: usize,
    pub distribution_type: DistributionType,
    pub addresses_file: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TokenConfig {
    pub batch_sender_address: Address,
    pub amount_per_address_low: f64,
    pub amount_per_address_high: f64,
}

#[derive(Debug, Deserialize)]
pub struct NFTConfig {
    pub token_address: Address,
    pub soulbound: bool,
    pub image_url: String,
}

#[derive(Debug, Deserialize)]
pub struct SwapperConfig {
    pub swapper_address: Address,
    pub max_swaps: u64,
}

// Unified configuration for all distribution types
#[derive(Debug, Deserialize)]
pub struct UnifiedConfig {
    pub core: CoreConfig,
    pub token: Option<TokenConfig>,
    pub nft: Option<NFTConfig>,
    pub swapper: Option<SwapperConfig>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: usize,
    pub method: String,
    pub params: Vec<String>,
}

pub fn read_config_file<T>(filename: &str) -> T
where
    T: for<'a> Deserialize<'a>,
{
    let mut file = File::open(format!("{}", filename)).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    // Parse the YAML into our Config struct
    let config: T = serde_yaml::from_str(&contents).unwrap();
    config
}

/// Poll for transaction receipt until it completes or times out using Alloy provider
pub async fn wait_for_transaction(
    rpc_url: String,
    tx_hash: String,
    timeout_seconds: u64,
) -> Result<Option<u64>> {
    info!("Waiting for transaction {} to be mined...", tx_hash);

    // Create an Alloy provider for just this polling operation
    let provider = ProviderBuilder::new().on_http(rpc_url.parse().unwrap());

    // Convert tx_hash string to B256 type
    let hash = TxHash::from_hex(tx_hash).unwrap();

    let poll_interval = Duration::from_millis(100);
    let timeout = Duration::from_secs(timeout_seconds);
    let start_time = std::time::Instant::now();

    while start_time.elapsed() < timeout {
        // Use Alloy provider to get transaction receipt
        match provider.get_transaction_receipt(hash).await {
            Ok(Some(receipt)) => {
                // Transaction has been mined
                let success = receipt.status();
                if success {
                    info!("Transaction successfully mined!");
                } else {
                    warn!(
                        "Transaction mined but failed! Status: {:?}",
                        receipt.status()
                    );
                }
                return Ok(receipt.block_number);
            }
            Ok(None) => {
                // Transaction not yet mined, continue polling
            }
            Err(e) => {
                error!("Error querying receipt: {:?}", e);
            }
        }

        // Wait before polling again
        sleep(poll_interval).await;
    }

    // Timeout reached
    warn!("Timeout reached while waiting for transaction");
    Ok(None)
}

pub async fn send_transactions_with_delay(
    txs: Vec<String>,
    http_client: Client,
    url: String,
    delay_ms: u64,
) {
    if delay_ms > 0 {
        sleep(Duration::from_millis(delay_ms)).await;
    }
    let requests: Vec<JsonRpcRequest> = txs
        .into_iter()
        .enumerate()
        .map(|(i, tx)| JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: i,
            method: "eth_sendRawTransaction".to_string(),
            params: vec![tx],
        })
        .collect();

    loop {
        match http_client
            .post(&url)
            .json(&requests)
            .timeout(Duration::from_secs(3))
            .send()
            .await
        {
            Ok(response) => {
                let x = response.text().await.unwrap();
                match serde_json::from_str::<Vec<Value>>(&x) {
                    Ok(body) => {
                        // If receive a valid RPC response, don't retry
                        for obj in body {
                            if obj.get("error").is_some() {
                                error!("URL {} RPC error: {:?}", url, obj);
                            }
                        }
                        return;
                    }
                    Err(e) => {
                        error!("URL {} Failed to parse response: {} {:?}", url, x, e);
                    }
                }
            }
            Err(e) => {
                error!("URL {} Batch request failed: {}", url, e);
            }
        }
    }
}

/// A trait for building transactions
#[async_trait]
pub trait Builder {
    /// Create and sign a transaction
    async fn build_transactions(&self, start_nonce_override: Option<u64>) -> Result<Vec<String>>;
}

/// A trait for distributing transactions
#[async_trait]
pub trait Distributor {
    /// Send transactions
    async fn send_transactions(
        &self,
        rpc_urls: Vec<String>,
        builder: Box<dyn Builder + Send + Sync>,
        options: DistributionOptions,
    ) -> Result<()>;
}

/// Options for transaction distribution
#[derive(Debug, Clone)]
pub struct DistributionOptions {
    pub rpc_urls: Vec<String>,
    pub rpc_batch_size: usize,
    pub target_tps: u64,
    pub wait_for_confirmation: bool,
    pub confirmation_timeout: u64,
}

impl Default for DistributionOptions {
    fn default() -> Self {
        Self {
            rpc_batch_size: 100,
            target_tps: 10,
            wait_for_confirmation: true,
            confirmation_timeout: 120,
            rpc_urls: Vec::new(),
        }
    }
}
