use proofface::content::canonicalizer::ContentCanonicalizer;
use proofface::crypto::hasher::ContentHasher;
use proofface::face::detector::FaceDetector;
use proofface::face::embedder::FaceEmbedder;
use proofface::face::similarity::cosine_similarity;
use proofface::models::DiscoveredContent;
use std::time::Instant;

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║             PROOFFACE 🦀 PERFORMANCE BENCHMARK          ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    // 1. SHA-256 Fingerprinting Benchmark
    let sample_payload = b"ProofFace Deterministic Canonical Content Benchmark Payload 2026";
    let iterations = 100_000;
    let start_hash = Instant::now();
    for _ in 0..iterations {
        let _ = ContentHasher::sha256(sample_payload);
    }
    let duration_hash = start_hash.elapsed();
    println!(
        "• SHA-256 Hashing          : {:?} total for {} ops ({:.2} ns/op)",
        duration_hash,
        iterations,
        (duration_hash.as_nanos() as f64) / (iterations as f64)
    );

    // 2. Canonicalization Benchmark
    let content = DiscoveredContent {
        source_url: "https://example.com/profiles/candidate_benchmark".into(),
        media_url: Some("https://example.com/images/candidate_face.jpg".into()),
        title: Some("Public Profile Face Verification".into()),
        snippet: Some("Performance profiling on canonical record generation".into()),
        content_hash: "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".into(),
        retrieved_at: chrono::Utc::now(),
    };

    let canon_iters = 50_000;
    let start_canon = Instant::now();
    for _ in 0..canon_iters {
        let _ = ContentCanonicalizer::canonicalize(&content).unwrap();
    }
    let duration_canon = start_canon.elapsed();
    println!(
        "• Canonicalization         : {:?} total for {} ops ({:.2} µs/op)",
        duration_canon,
        canon_iters,
        (duration_canon.as_micros() as f64) / (canon_iters as f64)
    );

    // 3. Face Detection & 128-dim Embedding Benchmark
    let raw_image_bytes = std::fs::read("samples/input.jpg").expect("samples/input.jpg required for benchmark");
    let detector = FaceDetector::new();
    let embedder = FaceEmbedder::new();

    let dynamic_img = detector.validate_and_load(&raw_image_bytes).unwrap();
    let detected_faces = detector.detect_faces(&dynamic_img).unwrap();
    let face = &detected_faces[0];

    let embed_iters = 500;
    let start_embed = Instant::now();
    for _ in 0..embed_iters {
        let _ = embedder.generate_embedding(face).unwrap();
    }
    let duration_embed = start_embed.elapsed();
    println!(
        "• Face Feature Embedding   : {:?} total for {} ops ({:.2} ms/op)",
        duration_embed,
        embed_iters,
        (duration_embed.as_secs_f64() * 1000.0) / (embed_iters as f64)
    );

    // 4. Cosine Similarity Vector Math Benchmark
    let emb1 = embedder.generate_embedding(face).unwrap();
    let emb2 = embedder.generate_embedding(face).unwrap();

    let sim_iters = 1_000_000;
    let start_sim = Instant::now();
    for _ in 0..sim_iters {
        let _ = cosine_similarity(&emb1, &emb2).unwrap();
    }
    let duration_sim = start_sim.elapsed();
    println!(
        "• Cosine Similarity (128d) : {:?} total for {} ops ({:.2} ns/op)",
        duration_sim,
        sim_iters,
        (duration_sim.as_nanos() as f64) / (sim_iters as f64)
    );

    println!("\n✓ Benchmark suite completed successfully.");
}
