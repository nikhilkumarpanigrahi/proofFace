use sha2::{Digest, Sha256};

pub struct ContentHasher;

impl ContentHasher {
    /// Computes the 32-byte SHA-256 digest of arbitrary bytes.
    pub fn sha256(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut output = [0u8; 32];
        output.copy_from_slice(&result);
        output
    }

    /// Computes SHA-256 and formats it as a lowercase 0x-prefixed hexadecimal string.
    pub fn sha256_hex(data: &[u8]) -> String {
        let bytes = Self::sha256(data);
        format!("0x{}", hex::encode(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_empty_bytes() {
        let hash = ContentHasher::sha256_hex(b"");
        assert_eq!(
            hash,
            "0xe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_sha256_deterministic() {
        let data = b"ProofFace Canonical Record v1";
        let h1 = ContentHasher::sha256(data);
        let h2 = ContentHasher::sha256(data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_sha256_one_byte_difference_changes_hash() {
        let data1 = b"ProofFace Test 1";
        let data2 = b"ProofFace Test 2";
        let h1 = ContentHasher::sha256(data1);
        let h2 = ContentHasher::sha256(data2);
        assert_ne!(h1, h2);
    }
}
