use chrono::Utc;
use image::{ImageBuffer, ImageFormat, Rgb};
use proofface::blockchain::contract::ContractEncoder;
use proofface::content::canonicalizer::ContentCanonicalizer;
use proofface::content::dedup::CandidateDeduplicator;
use proofface::crypto::hasher::ContentHasher;
use proofface::face::detector::FaceDetector;
use proofface::face::embedder::FaceEmbedder;
use proofface::face::similarity::{cosine_similarity, evaluate_similarity};
use proofface::models::{DiscoveredContent, FaceEmbedding, MatchConfidence};
use std::io::Cursor;

fn create_synthetic_test_image(r: u8, g: u8, b: u8) -> Vec<u8> {
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(120, 120, |_, _| Rgb([r, g, b]));
    let mut bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Jpeg)
        .unwrap();
    bytes
}

#[test]
fn test_image_validation_and_face_detection() {
    let detector = FaceDetector::new();

    // 1. Valid test image with skin tones
    let valid_bytes = create_synthetic_test_image(210, 160, 130);
    let loaded = detector.validate_and_load(&valid_bytes).unwrap();
    let faces = detector.detect_faces(&loaded).unwrap();
    assert!(!faces.is_empty(), "Expected at least one detected face");

    // 2. Corrupted image bytes
    let corrupt_bytes = vec![0x00, 0x12, 0x34, 0x56];
    assert!(detector.validate_and_load(&corrupt_bytes).is_err());

    // 3. Empty image bytes
    assert!(detector.validate_and_load(&[]).is_err());
}

#[test]
fn test_face_embedding_generation_and_normalization() {
    let detector = FaceDetector::new();
    let embedder = FaceEmbedder::new();

    let image_bytes = create_synthetic_test_image(200, 150, 120);
    let img = detector.validate_and_load(&image_bytes).unwrap();
    let faces = detector.detect_faces(&img).unwrap();
    let face = &faces[0];

    let embedding = embedder.generate_embedding(face).unwrap();
    assert_eq!(embedding.len(), 128);

    // Verify L2 unit norm
    let norm_sq: f32 = embedding.values.iter().map(|v| v * v).sum();
    assert!((norm_sq.sqrt() - 1.0).abs() < 1e-4);
}

#[test]
fn test_similarity_ranking_and_thresholds() {
    let emb_base = FaceEmbedding::new(vec![0.5; 128]);
    let emb_identical = FaceEmbedding::new(vec![0.5; 128]);
    let emb_different = FaceEmbedding::new(vec![-0.5; 128]);

    let sim_same = cosine_similarity(&emb_base, &emb_identical).unwrap();
    let sim_diff = cosine_similarity(&emb_base, &emb_different).unwrap();

    assert!((sim_same - 1.0).abs() < 1e-5);
    assert!((sim_diff - (-1.0)).abs() < 1e-5);

    assert_eq!(
        evaluate_similarity(sim_same, 0.75, 0.55),
        MatchConfidence::HighConfidence
    );
    assert_eq!(
        evaluate_similarity(sim_diff, 0.75, 0.55),
        MatchConfidence::NoMatch
    );
}

#[test]
fn test_content_canonicalization_and_tamper_detection() {
    let content_original = DiscoveredContent {
        source_url: "https://example.com/profiles/alice".into(),
        media_url: Some("https://example.com/photos/alice.jpg".into()),
        title: Some("Alice Public Profile".into()),
        snippet: Some("Verified contributor".into()),
        content_hash: "0xdeadbeef12345678".into(),
        retrieved_at: Utc::now(),
    };

    let (fp_orig, bytes_orig) = ContentCanonicalizer::fingerprint(&content_original).unwrap();
    assert_eq!(bytes_orig.len(), 32);
    assert!(fp_orig.starts_with("0x"));

    // Verify same canonical content with different timestamp produces identical fingerprint
    let content_same_diff_time = DiscoveredContent {
        source_url: "https://example.com/profiles/alice".into(),
        media_url: Some("https://example.com/photos/alice.jpg".into()),
        title: Some("Alice Public Profile".into()),
        snippet: Some("Verified contributor".into()),
        content_hash: "0xdeadbeef12345678".into(),
        retrieved_at: Utc::now() + chrono::Duration::days(10),
    };

    let (fp_same, _) = ContentCanonicalizer::fingerprint(&content_same_diff_time).unwrap();
    assert_eq!(fp_orig, fp_same, "Timestamps must be non-volatile in fingerprinting");

    // Alter title (tamper scenario)
    let content_tampered = DiscoveredContent {
        source_url: "https://example.com/profiles/alice".into(),
        media_url: Some("https://example.com/photos/alice.jpg".into()),
        title: Some("MALICIOUSLY ALTERED TITLE".into()),
        snippet: Some("Verified contributor".into()),
        content_hash: "0xdeadbeef12345678".into(),
        retrieved_at: Utc::now(),
    };

    let (fp_tampered, _) = ContentCanonicalizer::fingerprint(&content_tampered).unwrap();
    assert_ne!(fp_orig, fp_tampered, "Tampered content MUST yield different fingerprint");
}

#[test]
fn test_contract_abi_encoding_invariants() {
    let dummy_fp = ContentHasher::sha256(b"Test Proof Record");
    let test_url = "https://example.com/discovered";

    let encoded_register = ContractEncoder::encode_register_proof(&dummy_fp, test_url);
    assert!(!encoded_register.is_empty());
    assert_eq!(&encoded_register[4..36], &dummy_fp);

    let encoded_get = ContractEncoder::encode_get_proof(&dummy_fp);
    assert_eq!(encoded_get.len(), 36);
    assert_eq!(&encoded_get[4..36], &dummy_fp);
}

#[test]
fn test_candidate_deduplication_heuristics() {
    let mut dedup = CandidateDeduplicator::new();

    let u1 = "https://example.com/page?utm_campaign=spring&id=42";
    let u2 = "https://example.com/page?id=42";
    let u3 = "https://example.com/page?id=99";

    assert!(dedup.check_and_insert(u1));
    assert!(!dedup.check_and_insert(u2), "Tracking param variations must be deduplicated");
    assert!(dedup.check_and_insert(u3));
}
