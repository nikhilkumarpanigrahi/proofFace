use super::rlp::RlpEncoder;
use crate::error::{PipelineError, Result};
use k256::ecdsa::SigningKey;
use sha3::{Digest, Keccak256};

pub struct EvmSigner {
    signing_key: SigningKey,
    address: [u8; 20],
    address_hex: String,
}

pub struct UnsignedTx {
    pub nonce: u64,
    pub gas_price: u128,
    pub gas_limit: u64,
    pub to: Option<[u8; 20]>,
    pub value: u128,
    pub data: Vec<u8>,
    pub chain_id: u64,
}

impl EvmSigner {
    /// Creates a signer from a 32-byte hex string (with or without 0x prefix).
    pub fn from_hex_key(private_key_hex: &str) -> Result<Self> {
        let clean = private_key_hex.trim().trim_start_matches("0x");
        let bytes = hex::decode(clean).map_err(|e| {
            PipelineError::BlockchainRpcError {
                endpoint: "local_signer".into(),
                message: format!("Invalid hex private key: {e}"),
            }
        })?;

        if bytes.len() != 32 {
            return Err(PipelineError::BlockchainRpcError {
                endpoint: "local_signer".into(),
                message: format!("Private key must be 32 bytes (got {})", bytes.len()),
            });
        }

        let signing_key = SigningKey::from_bytes((&bytes[..]).into()).map_err(|e| {
            PipelineError::BlockchainRpcError {
                endpoint: "local_signer".into(),
                message: format!("Invalid ECDSA signing key: {e}"),
            }
        })?;

        // Derive Ethereum 20-byte address: Keccak256(uncompressed_public_key[1..])[12..32]
        let verifying_key = signing_key.verifying_key();
        let uncompressed = verifying_key.to_encoded_point(false);
        let pub_bytes = &uncompressed.as_bytes()[1..]; // skip 0x04 tag
        let mut hasher = Keccak256::new();
        hasher.update(pub_bytes);
        let hash = hasher.finalize();

        let mut address = [0u8; 20];
        address.copy_from_slice(&hash[12..32]);
        let address_hex = format!("0x{}", hex::encode(address));

        Ok(Self {
            signing_key,
            address,
            address_hex,
        })
    }

    pub fn address_hex(&self) -> &str {
        &self.address_hex
    }

    pub fn address(&self) -> &[u8; 20] {
        &self.address
    }

    /// Signs an EIP-155 transaction and returns `(raw_tx_bytes, tx_hash_hex)`.
    pub fn sign_transaction(&self, tx: &UnsignedTx) -> Result<(Vec<u8>, String)> {
        // EIP-155 unsigned payload: [nonce, gasprice, gaslimit, to, value, data, chainid, 0, 0]
        let mut items = Vec::with_capacity(9);
        items.push(RlpEncoder::encode_u64(tx.nonce));
        items.push(RlpEncoder::encode_u128(tx.gas_price));
        items.push(RlpEncoder::encode_u64(tx.gas_limit));

        match tx.to {
            Some(addr) => items.push(RlpEncoder::encode_bytes(&addr)),
            None => items.push(RlpEncoder::encode_bytes(&[])), // Contract deployment
        }

        items.push(RlpEncoder::encode_u128(tx.value));
        items.push(RlpEncoder::encode_bytes(&tx.data));
        items.push(RlpEncoder::encode_u64(tx.chain_id));
        items.push(RlpEncoder::encode_u64(0));
        items.push(RlpEncoder::encode_u64(0));

        let unsigned_rlp = RlpEncoder::encode_list(&items);

        let mut hasher = Keccak256::new();
        hasher.update(&unsigned_rlp);
        let signing_hash = hasher.finalize();

        // Sign prehash
        let (signature, recid) = self
            .signing_key
            .sign_prehash_recoverable(&signing_hash)
            .map_err(|e| PipelineError::BlockchainRpcError {
                endpoint: "local_signer".into(),
                message: format!("Failed to sign transaction: {e}"),
            })?;

        // EIP-155 v calculation: v = recid + chain_id * 2 + 35
        let v: u64 = (recid.to_byte() as u64) + tx.chain_id * 2 + 35;
        let r = signature.r().to_bytes();
        let s = signature.s().to_bytes();

        // EIP-155 signed payload: [nonce, gasprice, gaslimit, to, value, data, v, r, s]
        let mut signed_items = Vec::with_capacity(9);
        signed_items.push(RlpEncoder::encode_u64(tx.nonce));
        signed_items.push(RlpEncoder::encode_u128(tx.gas_price));
        signed_items.push(RlpEncoder::encode_u64(tx.gas_limit));

        match tx.to {
            Some(addr) => signed_items.push(RlpEncoder::encode_bytes(&addr)),
            None => signed_items.push(RlpEncoder::encode_bytes(&[])),
        }

        signed_items.push(RlpEncoder::encode_u128(tx.value));
        signed_items.push(RlpEncoder::encode_bytes(&tx.data));
        signed_items.push(RlpEncoder::encode_u64(v));
        signed_items.push(RlpEncoder::encode_bytes(&r));
        signed_items.push(RlpEncoder::encode_bytes(&s));

        let signed_rlp = RlpEncoder::encode_list(&signed_items);

        // Transaction hash is Keccak256 of the signed RLP stream
        let mut tx_hasher = Keccak256::new();
        tx_hasher.update(&signed_rlp);
        let tx_hash_bytes = tx_hasher.finalize();
        let tx_hash_hex = format!("0x{}", hex::encode(tx_hash_bytes));

        Ok((signed_rlp, tx_hash_hex))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_derivation() {
        // Known private key: 1 (0x000...01)
        let mut key_bytes = [0u8; 32];
        key_bytes[31] = 1;
        let signer = EvmSigner::from_hex_key(&hex::encode(key_bytes)).unwrap();
        // Public address for private key 1 is 0x7e5f4552091a69125d5dfcb7b8c2659029395bdf
        assert_eq!(
            signer.address_hex().to_lowercase(),
            "0x7e5f4552091a69125d5dfcb7b8c2659029395bdf"
        );
    }

    #[test]
    fn test_sign_transaction() {
        let mut key_bytes = [0u8; 32];
        key_bytes[31] = 1;
        let signer = EvmSigner::from_hex_key(&hex::encode(key_bytes)).unwrap();

        let tx = UnsignedTx {
            nonce: 0,
            gas_price: 20_000_000_000,
            gas_limit: 21_000,
            to: Some([0x11; 20]),
            value: 1_000_000_000_000_000_000,
            data: vec![],
            chain_id: 80002,
        };

        let (raw, tx_hash) = signer.sign_transaction(&tx).unwrap();
        assert!(!raw.is_empty());
        assert!(tx_hash.starts_with("0x"));
        assert_eq!(tx_hash.len(), 66);
    }
}
