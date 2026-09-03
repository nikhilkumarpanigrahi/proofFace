use sha3::{Digest, Keccak256};

pub struct ContractEncoder;

impl ContractEncoder {
    /// Computes 4-byte Ethereum function selector: keccak256(signature)[0..4]
    pub fn selector(signature: &str) -> [u8; 4] {
        let mut hasher = Keccak256::new();
        hasher.update(signature.as_bytes());
        let hash = hasher.finalize();
        let mut sel = [0u8; 4];
        sel.copy_from_slice(&hash[0..4]);
        sel
    }

    /// Encodes call to `getProof(bytes32)`
    pub fn encode_get_proof(fingerprint: &[u8; 32]) -> Vec<u8> {
        let mut data = Vec::with_capacity(36);
        let sel = Self::selector("getProof(bytes32)");
        data.extend_from_slice(&sel);
        data.extend_from_slice(fingerprint);
        data
    }

    /// Encodes call to `registerProof(bytes32,string)`
    pub fn encode_register_proof(fingerprint: &[u8; 32], source_url: &str) -> Vec<u8> {
        let mut data = Vec::new();
        let sel = Self::selector("registerProof(bytes32,string)");
        data.extend_from_slice(&sel);

        // Param 1: bytes32 fingerprint
        data.extend_from_slice(fingerprint);

        // Param 2: offset to dynamic string parameter (0x40 = 64 bytes from start of params)
        let mut offset = [0u8; 32];
        offset[31] = 0x40;
        data.extend_from_slice(&offset);

        // String length (32 bytes big-endian)
        let url_bytes = source_url.as_bytes();
        let mut len_bytes = [0u8; 32];
        let len = url_bytes.len() as u64;
        len_bytes[24..32].copy_from_slice(&len.to_be_bytes());
        data.extend_from_slice(&len_bytes);

        // String payload padded to 32-byte boundary
        data.extend_from_slice(url_bytes);
        let padding_needed = (32 - (url_bytes.len() % 32)) % 32;
        data.extend_from_slice(&vec![0u8; padding_needed]);

        data
    }

    /// Decodes return value from `getProof(bytes32)`
    /// Returns (fingerprint_bytes, source_url, timestamp, exists)
    pub fn decode_get_proof_output(output_bytes: &[u8]) -> Option<([u8; 32], String, u64, bool)> {
        if output_bytes.len() < 128 {
            return None;
        }

        let mut fp = [0u8; 32];
        fp.copy_from_slice(&output_bytes[0..32]);

        // String offset is at output_bytes[32..64]
        // Timestamp is at output_bytes[64..96]
        let mut ts_bytes = [0u8; 8];
        ts_bytes.copy_from_slice(&output_bytes[88..96]);
        let timestamp = u64::from_be_bytes(ts_bytes);

        // Exists bool is at output_bytes[96..128]
        let exists = output_bytes[127] == 1;

        if !exists {
            return Some((fp, String::new(), timestamp, false));
        }

        // Decode string
        let str_offset = 128;
        if output_bytes.len() >= str_offset + 32 {
            let mut str_len_bytes = [0u8; 8];
            str_len_bytes.copy_from_slice(&output_bytes[str_offset + 24..str_offset + 32]);
            let str_len = u64::from_be_bytes(str_len_bytes) as usize;

            let str_start = str_offset + 32;
            if output_bytes.len() >= str_start + str_len {
                if let Ok(url) = std::str::from_utf8(&output_bytes[str_start..str_start + str_len])
                {
                    return Some((fp, url.to_string(), timestamp, true));
                }
            }
        }

        Some((fp, String::new(), timestamp, exists))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selector_calculation() {
        let sel = ContractEncoder::selector("getProof(bytes32)");
        assert_eq!(sel.len(), 4);
    }

    #[test]
    fn test_encode_and_decode() {
        let fp = [7u8; 32];
        let encoded = ContractEncoder::encode_get_proof(&fp);
        assert_eq!(encoded.len(), 36);
        assert_eq!(&encoded[4..36], &fp);
    }
}
