use super::polygon::PolygonRegistry;
use super::signer::{EvmSigner, UnsignedTx};
use crate::error::{PipelineError, Result};
use serde_json::json;
use std::time::Duration;
use tracing::info;

pub const CONTENT_REGISTRY_BYTECODE_HEX: &str = include_str!("../../contracts/build/contracts_ContentRegistry_sol_ContentRegistry.bin");

pub struct ContractDeployer<'a> {
    registry: &'a PolygonRegistry,
}

impl<'a> ContractDeployer<'a> {
    pub fn new(registry: &'a PolygonRegistry) -> Self {
        Self { registry }
    }

    /// Deploys the ContentRegistry contract to Polygon Amoy.
    /// Returns the deployed contract address (0x...).
    pub async fn deploy(&self, private_key_hex: &str) -> Result<String> {
        let signer = EvmSigner::from_hex_key(private_key_hex)?;
        let deployer_address = signer.address_hex();

        info!(deployer = %deployer_address, "Initiating ContentRegistry deployment on Polygon Amoy");

        // 1. Fetch current nonce
        let nonce_payload = json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionCount",
            "params": [deployer_address, "pending"],
            "id": 1
        });
        let nonce_res = self.registry.send_rpc(&nonce_payload).await?;
        let nonce_str = nonce_res.as_str().ok_or_else(|| PipelineError::BlockchainRpcError {
            endpoint: "eth_getTransactionCount".into(),
            message: "Invalid nonce response".into(),
        })?;
        let nonce = u64::from_str_radix(nonce_str.trim_start_matches("0x"), 16).unwrap_or(0);

        // 2. Fetch current gas price
        let gas_payload = json!({
            "jsonrpc": "2.0",
            "method": "eth_gasPrice",
            "params": [],
            "id": 2
        });
        let gas_res = self.registry.send_rpc(&gas_payload).await?;
        let gas_price_str = gas_res.as_str().unwrap_or("0x77359400"); // fallback 2 Gwei
        let gas_price = u128::from_str_radix(gas_price_str.trim_start_matches("0x"), 16)
            .unwrap_or(2_000_000_000);
        // Add 20% buffer to gas price to ensure prompt mining
        let gas_price_with_buffer = (gas_price * 12) / 10;

        // 3. Decode bytecode
        let bytecode_clean = CONTENT_REGISTRY_BYTECODE_HEX.trim().trim_start_matches("0x");
        let bytecode = hex::decode(bytecode_clean).map_err(|e| {
            PipelineError::BlockchainRpcError {
                endpoint: "bytecode".into(),
                message: format!("Invalid bytecode hex: {e}"),
            }
        })?;

        // 4. Estimate gas limit or use standard deployment gas
        let gas_limit = 1_500_000u64;

        // 5. Sign deployment transaction (to: None)
        let tx = UnsignedTx {
            nonce,
            gas_price: gas_price_with_buffer,
            gas_limit,
            to: None,
            value: 0,
            data: bytecode,
            chain_id: self.registry.chain_id(),
        };

        let (raw_signed, tx_hash) = signer.sign_transaction(&tx)?;
        let raw_hex = format!("0x{}", hex::encode(raw_signed));

        println!("      Broadcasting deployment tx: {}", tx_hash);
        println!("      Deployer Address           : {}", deployer_address);
        println!("      Polygonscan Tracker        : https://amoy.polygonscan.com/tx/{}", tx_hash);

        // 6. Broadcast transaction
        let send_payload = json!({
            "jsonrpc": "2.0",
            "method": "eth_sendRawTransaction",
            "params": [raw_hex],
            "id": 3
        });
        self.registry.send_rpc(&send_payload).await?;

        // 7. Wait for receipt to get contractAddress
        println!("      Waiting for block confirmation on Polygon Amoy...");
        let receipt_payload = json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionReceipt",
            "params": [tx_hash],
            "id": 4
        });

        for _ in 0..30 {
            tokio::time::sleep(Duration::from_secs(2)).await;
            if let Ok(receipt) = self.registry.send_rpc(&receipt_payload).await {
                if !receipt.is_null() {
                    if let Some(contract_addr) = receipt.get("contractAddress").and_then(|v| v.as_str()) {
                        let status = receipt.get("status").and_then(|s| s.as_str()).unwrap_or("0x1");
                        if status == "0x1" {
                            println!("      ✓ Contract Deployed Successfully!");
                            println!("      Contract Address: {}", contract_addr);
                            println!("      Polygonscan Link: https://amoy.polygonscan.com/address/{}", contract_addr);
                            return Ok(contract_addr.to_string());
                        } else {
                            return Err(PipelineError::BlockchainRpcError {
                                endpoint: "eth_getTransactionReceipt".into(),
                                message: "Transaction reverted on chain".into(),
                            });
                        }
                    }
                }
            }
        }

        Err(PipelineError::BlockchainRpcError {
            endpoint: "eth_getTransactionReceipt".into(),
            message: "Deployment transaction confirmation timed out after 60s".into(),
        })
    }
}
