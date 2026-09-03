use crate::crypto::hasher::ContentHasher;
use crate::error::{PipelineError, Result};
use crate::models::DiscoveredContent;
use serde_json::json;

pub struct ContentCanonicalizer;

impl ContentCanonicalizer {
    /// Produces a strict RFC 8785 JSON Canonicalization Scheme (JCS) deterministic representation.
    /// Excludes volatile non-deterministic timestamps and formats fields deterministically.
    pub fn canonicalize(content: &DiscoveredContent) -> Result<Vec<u8>> {
        // Strip volatile fields and normalize strings
        let normalized_source = content.source_url.trim().to_lowercase();
        let normalized_media = content
            .media_url
            .as_deref()
            .map(|s| s.trim().to_lowercase());
        let normalized_title = content.title.as_deref().map(|s| s.trim());
        let normalized_snippet = content.snippet.as_deref().map(|s| s.trim());
        let normalized_hash = content.content_hash.trim().to_lowercase();

        // Build deterministic structure
        let mut map = serde_json::Map::new();
        map.insert("content_hash".to_string(), json!(normalized_hash));
        map.insert("media_url".to_string(), json!(normalized_media));
        map.insert("snippet".to_string(), json!(normalized_snippet));
        map.insert("source_url".to_string(), json!(normalized_source));
        map.insert("title".to_string(), json!(normalized_title));
        map.insert("version".to_string(), json!("1.0"));

        let canonical_value = serde_json::Value::Object(map);

        // Strict RFC 8785 JSON Canonicalization Scheme (JCS) serialization
        serde_jcs::to_vec(&canonical_value).map_err(|e| {
            PipelineError::CanonicalizationFailed(format!("RFC 8785 serialization error: {e}"))
        })
    }

    /// Generates the deterministic SHA-256 fingerprint for discovered content over RFC 8785 bytes.
    pub fn fingerprint(content: &DiscoveredContent) -> Result<(String, [u8; 32])> {
        let canonical_bytes = Self::canonicalize(content)?;
        let bytes = ContentHasher::sha256(&canonical_bytes);
        let hex_str = format!("0x{}", hex::encode(bytes));
        Ok((hex_str, bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_canonicalization_is_deterministic() {
        let content1 = DiscoveredContent {
            source_url: "https://example.com/post/42 ".into(),
            media_url: Some("https://example.com/images/face.jpg ".into()),
            title: Some("Public Profile".into()),
            snippet: Some("Discovered Face".into()),
            content_hash: "0x123abc".into(),
            retrieved_at: Utc::now(),
        };

        let content2 = DiscoveredContent {
            source_url: "https://example.com/post/42".into(),
            media_url: Some("https://example.com/images/face.jpg".into()),
            title: Some("Public Profile".into()),
            snippet: Some("Discovered Face".into()),
            content_hash: "0x123abc".into(),
            retrieved_at: Utc::now() + chrono::Duration::hours(2), // Different timestamp
        };

        let (fp1, _) = ContentCanonicalizer::fingerprint(&content1).unwrap();
        let (fp2, _) = ContentCanonicalizer::fingerprint(&content2).unwrap();

        // Fingerprints must match because volatile timestamps are excluded
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_modified_content_produces_tampered_fingerprint() {
        let original = DiscoveredContent {
            source_url: "https://example.com/post/42".into(),
            media_url: Some("https://example.com/images/face.jpg".into()),
            title: Some("Public Profile".into()),
            snippet: Some("Discovered Face".into()),
            content_hash: "0x123abc".into(),
            retrieved_at: Utc::now(),
        };

        let tampered = DiscoveredContent {
            source_url: "https://example.com/post/42".into(),
            media_url: Some("https://example.com/images/face.jpg".into()),
            title: Some("TAMPERED Profile".into()),
            snippet: Some("Discovered Face".into()),
            content_hash: "0x123abc".into(),
            retrieved_at: Utc::now(),
        };

        let (fp1, _) = ContentCanonicalizer::fingerprint(&original).unwrap();
        let (fp2, _) = ContentCanonicalizer::fingerprint(&tampered).unwrap();

        assert_ne!(fp1, fp2);
    }
}
