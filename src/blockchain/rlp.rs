/// Minimal and complete Recursive Length Prefix (RLP) serialization for EVM transactions.

pub struct RlpEncoder;

impl RlpEncoder {
    /// Encodes a raw byte slice according to RLP rules.
    pub fn encode_bytes(bytes: &[u8]) -> Vec<u8> {
        if bytes.len() == 1 && bytes[0] < 0x80 {
            return bytes.to_vec();
        }

        if bytes.len() <= 55 {
            let mut out = Vec::with_capacity(1 + bytes.len());
            out.push(0x80 + bytes.len() as u8);
            out.extend_from_slice(bytes);
            out
        } else {
            let len_bytes = Self::to_be_bytes_trimmed(bytes.len() as u64);
            let mut out = Vec::with_capacity(1 + len_bytes.len() + bytes.len());
            out.push(0xb7 + len_bytes.len() as u8);
            out.extend_from_slice(&len_bytes);
            out.extend_from_slice(bytes);
            out
        }
    }

    /// Encodes an unsigned integer (u64) according to RLP integer rules.
    /// Note: 0 is encoded as an empty byte slice (0x80).
    pub fn encode_u64(val: u64) -> Vec<u8> {
        if val == 0 {
            return vec![0x80];
        }
        let bytes = Self::to_be_bytes_trimmed(val);
        Self::encode_bytes(&bytes)
    }

    /// Encodes an unsigned 128-bit integer (e.g. gas price, value in wei).
    pub fn encode_u128(val: u128) -> Vec<u8> {
        if val == 0 {
            return vec![0x80];
        }
        let be = val.to_be_bytes();
        let trimmed = Self::strip_leading_zeros(&be);
        Self::encode_bytes(trimmed)
    }

    /// Encodes a list of already-encoded RLP items into an RLP list.
    pub fn encode_list(items: &[Vec<u8>]) -> Vec<u8> {
        let payload_len: usize = items.iter().map(|it| it.len()).sum();
        let mut out = Vec::new();

        if payload_len <= 55 {
            out.push(0xc0 + payload_len as u8);
        } else {
            let len_bytes = Self::to_be_bytes_trimmed(payload_len as u64);
            out.push(0xf7 + len_bytes.len() as u8);
            out.extend_from_slice(&len_bytes);
        }

        for item in items {
            out.extend_from_slice(item);
        }

        out
    }

    fn to_be_bytes_trimmed(val: u64) -> Vec<u8> {
        let be = val.to_be_bytes();
        let trimmed = Self::strip_leading_zeros(&be);
        trimmed.to_vec()
    }

    fn strip_leading_zeros(bytes: &[u8]) -> &[u8] {
        let mut start = 0;
        while start < bytes.len() && bytes[start] == 0 {
            start += 1;
        }
        &bytes[start..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rlp_empty_string() {
        assert_eq!(RlpEncoder::encode_bytes(b""), vec![0x80]);
    }

    #[test]
    fn test_rlp_single_byte() {
        assert_eq!(RlpEncoder::encode_bytes(&[0x0f]), vec![0x0f]);
        assert_eq!(RlpEncoder::encode_bytes(&[0x7f]), vec![0x7f]);
        assert_eq!(RlpEncoder::encode_bytes(&[0x80]), vec![0x81, 0x80]);
    }

    #[test]
    fn test_rlp_short_string() {
        assert_eq!(RlpEncoder::encode_bytes(b"dog"), vec![0x83, b'd', b'o', b'g']);
    }

    #[test]
    fn test_rlp_empty_list() {
        assert_eq!(RlpEncoder::encode_list(&[]), vec![0xc0]);
    }

    #[test]
    fn test_rlp_u64() {
        assert_eq!(RlpEncoder::encode_u64(0), vec![0x80]);
        assert_eq!(RlpEncoder::encode_u64(15), vec![0x0f]);
        assert_eq!(RlpEncoder::encode_u64(1024), vec![0x82, 0x04, 0x00]);
    }

    #[test]
    fn test_rlp_string_list() {
        let cat = RlpEncoder::encode_bytes(b"cat");
        let dog = RlpEncoder::encode_bytes(b"dog");
        let list = RlpEncoder::encode_list(&[cat, dog]);
        assert_eq!(list, vec![0xc8, 0x83, b'c', b'a', b't', 0x83, b'd', b'o', b'g']);
    }
}
