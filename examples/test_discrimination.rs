use proofface::face::detector::FaceDetector;
use proofface::face::embedder::FaceEmbedder;
use proofface::face::similarity::cosine_similarity;

fn main() {
    let detector = FaceDetector::new();
    let embedder = FaceEmbedder::new();

    let path1 = std::path::Path::new("samples/my_test.jpg");
    let path2 = std::path::Path::new("samples/ronaldo.jpg");

    let (img1_bytes, img2_bytes, label) = if path1.exists() && path2.exists() {
        (
            std::fs::read(path1).unwrap(),
            std::fs::read(path2).unwrap(),
            "Ishan Sharma and Cristiano Ronaldo",
        )
    } else if std::path::Path::new("samples/input.jpg").exists() {
        println!("Note: samples/my_test.jpg and samples/ronaldo.jpg not found.");
        println!("Evaluating discrimination on available sample images...");
        (
            std::fs::read("samples/input.jpg").unwrap(),
            std::fs::read("samples/input.jpg").unwrap(),
            "Identical input face sample (self-similarity baseline)",
        )
    } else {
        eprintln!(
            "No sample images found in samples/. Run `cargo run --example generate_samples` first."
        );
        return;
    };

    let loaded1 = detector.validate_and_load(&img1_bytes).unwrap();
    let loaded2 = detector.validate_and_load(&img2_bytes).unwrap();

    let faces1 = detector.detect_faces(&loaded1).unwrap();
    let faces2 = detector.detect_faces(&loaded2).unwrap();

    let emb1 = embedder.generate_embedding(&faces1[0]).unwrap();
    let emb2 = embedder.generate_embedding(&faces2[0]).unwrap();

    let sim = cosine_similarity(&emb1, &emb2).unwrap();
    println!("Similarity between {label}: {:.4}", sim);
}
