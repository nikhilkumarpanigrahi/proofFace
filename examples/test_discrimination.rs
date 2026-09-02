use proofface::face::detector::FaceDetector;
use proofface::face::embedder::FaceEmbedder;
use proofface::face::similarity::cosine_similarity;

fn main() {
    let detector = FaceDetector::new();
    let embedder = FaceEmbedder::new();

    let img1_bytes = std::fs::read("samples/my_test.jpg").unwrap();
    let img2_bytes = std::fs::read("samples/ronaldo.jpg").unwrap();

    let loaded1 = detector.validate_and_load(&img1_bytes).unwrap();
    let loaded2 = detector.validate_and_load(&img2_bytes).unwrap();

    let faces1 = detector.detect_faces(&loaded1).unwrap();
    let faces2 = detector.detect_faces(&loaded2).unwrap();

    let emb1 = embedder.generate_embedding(&faces1[0]).unwrap();
    let emb2 = embedder.generate_embedding(&faces2[0]).unwrap();

    let sim = cosine_similarity(&emb1, &emb2).unwrap();
    println!("Similarity between Ishan Sharma and Cristiano Ronaldo: {:.4}", sim);
}
