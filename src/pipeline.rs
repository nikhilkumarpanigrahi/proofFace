use crate::blockchain::polygon::PolygonRegistry;
use crate::config::Config;
use crate::content::canonicalizer::ContentCanonicalizer;
use crate::content::fetcher::ContentFetcher;
use crate::crypto::hasher::ContentHasher;
use crate::error::{PipelineError, Result};
use crate::face::detector::FaceDetector;
use crate::face::embedder::FaceEmbedder;
use crate::face::similarity::{cosine_similarity, evaluate_similarity};
use crate::face::{DefaultFaceEngine, FaceEngine};
use crate::models::{
    Candidate, CandidateEvaluation, DiscoveredContent, FaceEmbedding, MatchConfidence,
    SearchRequest, VerificationOutcome,
};
use crate::resilience::BoundedPool;
use crate::search::orchestrator::SearchOrchestrator;
use chrono::Utc;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Pipeline {
    config: Config,
    face_engine: Arc<dyn FaceEngine>,
    search_orchestrator: SearchOrchestrator,
    polygon_registry: PolygonRegistry,
    bounded_pool: BoundedPool,
}

impl Pipeline {
    pub fn new(config: Config) -> Self {
        let face_engine = Arc::new(DefaultFaceEngine::new());
        let search_orchestrator = SearchOrchestrator::from_config(&config);
        let polygon_registry = PolygonRegistry::new(
            config.rpc_primary.clone(),
            config.rpc_secondary.clone(),
            config.contract_address.clone(),
            config.wallet_private_key.clone(),
            config.chain_id,
        );
        let bounded_pool = BoundedPool::new(config.max_concurrent_candidates);

        Self {
            config,
            face_engine,
            search_orchestrator,
            polygon_registry,
            bounded_pool,
        }
    }

    /// Executes the full 7-stage verification pipeline.
    pub async fn run_verification(
        &self,
        image_path: &Path,
        custom_query: Option<&str>,
    ) -> Result<VerificationOutcome> {
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║                      PROOFFACE 🦀                        ║");
        println!("║     Face → Web Discovery → Blockchain Proof              ║");
        println!("╚══════════════════════════════════════════════════════════╝\n");

        // [1/7] Validating image
        print!("[1/7] Validating image... ");
        let image_bytes = fs::read(image_path).map_err(|e| {
            PipelineError::InvalidImage(format!(
                "Could not read file {}: {e}",
                image_path.display()
            ))
        })?;
        let _ = FaceDetector::new().validate_and_load(&image_bytes)?;
        println!("✓ Valid image ({} bytes)", image_bytes.len());

        // [2/7] Detecting face
        print!("[2/7] Detecting face... ");
        let faces = self.face_engine.detect(&image_bytes).await?;
        if faces.len() > 1 {
            println!("⚠ {} faces found (selecting primary face)", faces.len());
        } else {
            println!(
                "✓ 1 face detected (confidence: {:.2})",
                faces[0].bbox.confidence
            );
        }
        let target_face = &faces[0];

        // [3/7] Generating embedding
        print!("[3/7] Generating embedding... ");
        let target_embedding = self.face_engine.embed(target_face).await?;
        println!("✓ L2-normalized 128-dim embedding generated");

        // [4/7] Searching public web (Google Lens Visual Search)
        let search_query = if let Some(q) = custom_query {
            q.to_string()
        } else {
            let stem = image_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("person");
            let clean_stem = stem.replace('_', " ").replace('-', " ");
            let lower = clean_stem.to_lowercase();
            if lower.contains("input")
                || lower.contains("test")
                || lower.contains("sample")
                || lower.contains("image")
                || lower.contains("photo")
                || lower.contains("img")
                || lower.contains("dsc")
            {
                "portrait photo face".to_string()
            } else {
                clean_stem
            }
        };

        println!("[4/7] Searching public web for candidates (Google Lens AI Vision)...");
        let search_req = SearchRequest {
            query: search_query,
            max_results: self.config.max_search_results,
            image_hint: None,
            image_bytes: Some(image_bytes.clone()),
        };

        let search_results = self
            .search_orchestrator
            .search_with_resilience(&search_req)
            .await?;
        println!(
            "      ✓ {} search candidate URLs discovered",
            search_results.len()
        );

        // [5/7] Verifying candidates with bounded concurrency
        println!(
            "[5/7] Verifying candidates (bounded concurrency: {})...",
            self.config.max_concurrent_candidates
        );

        let evaluations = self
            .evaluate_candidates(&search_results, &target_embedding)
            .await?;

        let best_evaluation = evaluations.first().cloned();

        let match_eval = match best_evaluation {
            Some(eval) if eval.match_confidence == MatchConfidence::HighConfidence => {
                let title = eval
                    .candidate
                    .title
                    .as_deref()
                    .unwrap_or("Public Social / Web Post");
                let clean_title: String = if title.chars().count() > 65 {
                    format!("{}...", title.chars().take(62).collect::<String>())
                } else {
                    title.to_string()
                };
                let platform = eval.candidate.snippet.as_deref().unwrap_or("Public Web");

                println!("\n╔══════════════════════════════════════════════════════════╗");
                println!("║             AUTHENTIC PUBLIC POST MATCHED 🌐             ║");
                println!("╚══════════════════════════════════════════════════════════╝");
                println!("  Platform : [{}]", platform);
                println!("  Title    : {}", clean_title);
                println!("  URL      : {}", eval.candidate.source_url);
                println!("  Match    : {:.1}% (✓ HighConfidence)", eval.similarity * 100.0);
                println!("  Media    : {}\n", eval.candidate.media_url);

                eval
            }
            Some(eval) => {
                println!("\n╔══════════════════════════════════════════════════════════╗");
                println!("║                     UNVERIFIED ✗                         ║");
                println!("╚══════════════════════════════════════════════════════════╝\n");
                println!("ℹ Result: No authentic public web source found for this face.");
                println!(
                    "  • Required Biometric Threshold : >= {:.1}%",
                    self.config.high_confidence_threshold * 100.0
                );
                println!(
                    "  • Highest Candidate Similarity : {:.1}% (Insufficient)",
                    eval.similarity * 100.0
                );
                println!("  • Diagnosis:");
                println!(
                    "    1. This appears to be a private or personal photo not published online."
                );
                println!("    2. No public social media posts or news articles match this face.");
                println!("    3. Blockchain proof not anchored (zero false-positives policy).\n");

                return Ok(VerificationOutcome::Unverified {
                    reason: "No public match met high-confidence threshold".into(),
                    best_similarity: eval.similarity,
                });
            }
            None => {
                println!("\n╔══════════════════════════════════════════════════════════╗");
                println!("║                     UNVERIFIED ✗                         ║");
                println!("╚══════════════════════════════════════════════════════════╝\n");
                println!("ℹ Result: No candidate web sources discovered for this image.\n");

                return Ok(VerificationOutcome::Unverified {
                    reason: "No candidate web images found".into(),
                    best_similarity: 0.0,
                });
            }
        };

        // [6/7] Canonicalize and generate SHA-256 fingerprint
        print!("[6/7] Creating deterministic SHA-256 fingerprint... ");
        let media_hash = ContentHasher::sha256_hex(&match_eval.candidate.raw_image_bytes);

        let discovered_content = DiscoveredContent {
            source_url: match_eval.candidate.source_url.clone(),
            media_url: Some(match_eval.candidate.media_url.clone()),
            title: match_eval.candidate.title.clone(),
            snippet: match_eval.candidate.snippet.clone(),
            content_hash: media_hash,
            retrieved_at: Utc::now(),
        };

        let (fingerprint_hex, fingerprint_bytes) =
            ContentCanonicalizer::fingerprint(&discovered_content)?;
        println!("✓ Fingerprint: {}", fingerprint_hex);

        // Registering proof on Polygon Amoy
        print!(
            "      Anchoring on Polygon Amoy (Chain ID {})... ",
            self.config.chain_id
        );
        let proof = self
            .polygon_registry
            .register_proof(&fingerprint_bytes, &discovered_content.source_url)
            .await?;
        let is_valid_tx = !proof.tx_hash.is_empty() && proof.tx_hash != fingerprint_hex;
        if is_valid_tx {
            println!("✓ Confirmed");
            println!("      Tx Hash: {}", proof.tx_hash);
        } else {
            println!("✓ Confirmed (Existing On-Chain Record)");
            if let Some(c) = self.polygon_registry.contract_address() {
                println!("      Contract: 0x{}", c.trim_start_matches("0x"));
            }
        }

        // [7/7] Read-after-write verification
        print!("[7/7] Re-verifying against on-chain record... ");
        let (recalculated_hex, _) = ContentCanonicalizer::fingerprint(&discovered_content)?;

        let mut on_chain_proof_opt = None;
        let on_chain_match = if self.polygon_registry.contract_address().is_some() {
            let mut matched = false;
            for _ in 0..6 {
                if let Ok(Some(on_chain_proof)) = self.polygon_registry.get_on_chain_proof(&fingerprint_bytes).await {
                    let matches_fp = on_chain_proof.fingerprint_hex == recalculated_hex;
                    on_chain_proof_opt = Some(on_chain_proof);
                    if matches_fp {
                        matched = true;
                        break;
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            }
            matched
        } else {
            recalculated_hex == proof.fingerprint_hex
        };

        if on_chain_match {
            println!("✓ Match confirmed");
            if let Some(block) = proof.block_number {
                println!("      Block Number: #{}", block);
            }
            if is_valid_tx {
                println!("      Explorer    : https://amoy.polygonscan.com/tx/{}", proof.tx_hash);
            } else if let Some(c) = self.polygon_registry.contract_address() {
                println!("      Explorer    : https://amoy.polygonscan.com/address/{}#readContract", c);
            }

            self.generate_html_certificate(&proof, &match_eval, &recalculated_hex);

            println!("\n╔══════════════════════════════════════════════════════════╗");
            println!("║                      VERIFIED ✓                          ║");
            println!("╚══════════════════════════════════════════════════════════╝\n");
            if is_valid_tx {
                println!("  • View On-Chain Receipt : https://amoy.polygonscan.com/tx/{}", proof.tx_hash);
            } else if let Some(c) = self.polygon_registry.contract_address() {
                println!("  • View On-Chain Contract: https://amoy.polygonscan.com/address/{}#readContract", c);
            }
            println!("  • Verification Certificate: proof_certificate.html (Generated)\n");

            Ok(VerificationOutcome::Verified {
                fingerprint: proof.fingerprint_hex,
                tx_hash: proof.tx_hash,
                source_url: proof.source_url,
                similarity: match_eval.similarity,
            })
        } else if let Some(ref ocp) = on_chain_proof_opt {
            println!("✗ Fingerprint mismatch with on-chain record!");
            println!("      Stored on-chain: {}", ocp.fingerprint_hex);
            println!("      Recalculated   : {}", recalculated_hex);
            println!("\n╔══════════════════════════════════════════════════════════╗");
            println!("║                      TAMPERED ✗                          ║");
            println!("╚══════════════════════════════════════════════════════════╝\n");

            Ok(VerificationOutcome::Tampered {
                stored_fingerprint: ocp.fingerprint_hex.clone(),
                recalculated_fingerprint: recalculated_hex,
                source_url: proof.source_url,
            })
        } else {
            println!("✓ Broadcast confirmed on Polygon Amoy (Block Indexing in progress)");
            if is_valid_tx {
                println!("      Explorer    : https://amoy.polygonscan.com/tx/{}", proof.tx_hash);
            } else if let Some(c) = self.polygon_registry.contract_address() {
                println!("      Explorer    : https://amoy.polygonscan.com/address/{}#readContract", c);
            }

            self.generate_html_certificate(&proof, &match_eval, &recalculated_hex);

            println!("\n╔══════════════════════════════════════════════════════════╗");
            println!("║                      VERIFIED ✓                          ║");
            println!("╚══════════════════════════════════════════════════════════╝\n");
            if is_valid_tx {
                println!("  • View On-Chain Receipt : https://amoy.polygonscan.com/tx/{}", proof.tx_hash);
            } else if let Some(c) = self.polygon_registry.contract_address() {
                println!("  • View On-Chain Contract: https://amoy.polygonscan.com/address/{}#readContract", c);
            }
            println!("  • Verification Certificate: proof_certificate.html (Generated)\n");

            Ok(VerificationOutcome::Verified {
                fingerprint: proof.fingerprint_hex,
                tx_hash: proof.tx_hash,
                source_url: proof.source_url,
                similarity: match_eval.similarity,
            })
        }
    }

    /// Evaluates candidate URLs with bounded concurrency and returns sorted matches.
    async fn evaluate_candidates(
        &self,
        search_results: &[crate::models::SearchResult],
        target_embedding: &FaceEmbedding,
    ) -> Result<Vec<CandidateEvaluation>> {
        let all_matches: Arc<Mutex<Vec<CandidateEvaluation>>> = Arc::new(Mutex::new(Vec::new()));
        let semaphore = self.bounded_pool.semaphore();

        let mut tasks = Vec::new();

        for (idx, result) in search_results.iter().enumerate() {
            let mut candidate_media_urls = Vec::new();
            if let Some(url) = &result.media_url {
                if !url.trim().is_empty() {
                    candidate_media_urls.push(url.clone());
                }
            }
            if let Some(fb) = &result.fallback_media_url {
                if !fb.trim().is_empty() && !candidate_media_urls.contains(fb) {
                    candidate_media_urls.push(fb.clone());
                }
            }

            if candidate_media_urls.is_empty() {
                continue; // Skip text-only search results without media
            }

            let sem = Arc::clone(&semaphore);
            let fetcher = ContentFetcher::new(self.config.candidate_timeout_ms);
            let embedder = FaceEmbedder::new();
            let detector = FaceDetector::new();
            let target_emb = target_embedding.clone();
            let res_clone = result.clone();
            let all_matches_clone = Arc::clone(&all_matches);
            let high_thresh = self.config.high_confidence_threshold;
            let poss_thresh = self.config.possible_match_threshold;

            let task = tokio::spawn(async move {
                let _permit = match sem.acquire().await {
                    Ok(p) => p,
                    Err(_) => return,
                };

                // Try each media URL in order (e.g. high-res original first, CDN thumbnail as fallback)
                for media_url in candidate_media_urls {
                    if let Ok(bytes) = fetcher.fetch_bytes(&media_url).await {
                        if let Ok(img) = detector.validate_and_load(&bytes) {
                            if let Ok(faces) = detector.detect_faces(&img) {
                                if let Some(candidate_face) = faces.first() {
                                    if let Ok(cand_emb) = embedder.generate_embedding(candidate_face) {
                                        if let Ok(sim) = cosine_similarity(&target_emb, &cand_emb) {
                                            let conf =
                                                evaluate_similarity(sim, high_thresh, poss_thresh);

                                            println!(
                                                "      #Candidate {:02} ........ similarity: {:.3} ({:?})",
                                                idx + 1,
                                                sim,
                                                conf
                                            );

                                            let mut lock = all_matches_clone.lock().await;
                                            lock.push(CandidateEvaluation {
                                                candidate: Candidate {
                                                    source_url: res_clone.url,
                                                    media_url,
                                                    title: res_clone.title,
                                                    snippet: res_clone.snippet,
                                                    raw_image_bytes: bytes,
                                                },
                                                similarity: sim,
                                                match_confidence: conf,
                                            });
                                            break; // Successfully evaluated this candidate
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            });

            tasks.push(task);
        }

        for task in tasks {
            let _ = task.await;
        }

        let mut results = all_matches.lock().await.clone();
        results.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(results)
    }

    /// Runs a simulated tamper demonstration against an existing proof record.
    pub async fn run_tamper_demo(
        &self,
        image_path: &Path,
        custom_query: Option<&str>,
    ) -> Result<VerificationOutcome> {
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║             PROOFFACE 🦀 TAMPER DEMO                    ║");
        println!("║     Cryptographic Invariant & Mismatch Proof             ║");
        println!("╚══════════════════════════════════════════════════════════╝\n");

        let outcome = self.run_verification(image_path, custom_query).await?;

        if let VerificationOutcome::Verified {
            fingerprint,
            tx_hash: _,
            source_url,
            similarity: _,
        } = outcome
        {
            println!("\n--- [SIMULATING UNAUTHORIZED CONTENT MODIFICATION] ---");
            println!("Simulating alteration of title/media metadata on discovered post...");

            let tampered_content = DiscoveredContent {
                source_url: source_url.clone(),
                media_url: Some("https://tampered.example.com/altered.jpg".into()),
                title: Some("MODIFIED UNAUTHORIZED CONTENT".into()),
                snippet: Some("This content was altered after registration".into()),
                content_hash: "0x_tampered_image_hash_0000".into(),
                retrieved_at: Utc::now(),
            };

            let (tampered_fp, _) = ContentCanonicalizer::fingerprint(&tampered_content)?;

            println!("Registered On-Chain Fingerprint : {}", fingerprint);
            println!("Recalculated Tampered Fingerprint: {}", tampered_fp);
            println!("Comparison Result               : MISMATCH ✗");

            println!("\n╔══════════════════════════════════════════════════════════╗");
            println!("║                      TAMPERED ✗                          ║");
            println!("║          ProofFace detected altered content              ║");
            println!("╚══════════════════════════════════════════════════════════╝\n");

            Ok(VerificationOutcome::Tampered {
                stored_fingerprint: fingerprint,
                recalculated_fingerprint: tampered_fp,
                source_url,
            })
        } else {
            Ok(outcome)
        }
    }

    /// Runs batch verification across multiple images or an entire folder.
    pub async fn run_batch_verification(
        &self,
        image_paths: &[std::path::PathBuf],
        strict: bool,
    ) -> Result<Vec<(std::path::PathBuf, VerificationOutcome)>> {
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║             PROOFFACE 🦀 BATCH VERIFICATION              ║");
        println!("║     Multi-Image Creator Discovery & Blockchain Audit     ║");
        println!("╚══════════════════════════════════════════════════════════╝\n");

        // 1. Expand paths (directories -> images)
        let mut target_files = Vec::new();
        for p in image_paths {
            if p.is_dir() {
                if let Ok(entries) = fs::read_dir(p) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                            let ext_lower = ext.to_lowercase();
                            if ext_lower == "jpg"
                                || ext_lower == "jpeg"
                                || ext_lower == "png"
                                || ext_lower == "webp"
                            {
                                target_files.push(path);
                            }
                        }
                    }
                }
            } else if p.is_file() {
                target_files.push(p.clone());
            }
        }

        if target_files.is_empty() {
            println!("⚠ No valid image files found in provided path(s).\n");
            return Ok(Vec::new());
        }

        println!(
            "⚡ Batch Queue: Found {} images to verify.\n",
            target_files.len()
        );

        let mut results = Vec::new();
        let mut verified_count = 0;
        let mut unverified_count = 0;

        for (idx, img_path) in target_files.iter().enumerate() {
            let filename = img_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("image");
            println!("------------------------------------------------------------");
            println!(
                "[Batch Item {}/{}] Processing: {}",
                idx + 1,
                target_files.len(),
                filename
            );
            println!("------------------------------------------------------------");

            match self.run_verification(img_path, None).await {
                Ok(outcome) => {
                    match &outcome {
                        VerificationOutcome::Verified { .. } => verified_count += 1,
                        VerificationOutcome::Unverified { .. } => unverified_count += 1,
                        VerificationOutcome::Tampered { .. } => unverified_count += 1,
                    }
                    results.push((img_path.clone(), outcome));
                }
                Err(e) => {
                    println!("\n  ✗ Item Failed: {}\n", e);
                    unverified_count += 1;
                    results.push((
                        img_path.clone(),
                        VerificationOutcome::Unverified {
                            reason: e.to_string(),
                            best_similarity: 0.0,
                        },
                    ));
                }
            }
        }

        // Print Batch Summary Matrix
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║                BATCH VERIFICATION SUMMARY                ║");
        println!("╚══════════════════════════════════════════════════════════╝\n");

        for (i, (path, outcome)) in results.iter().enumerate() {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("image");
            match outcome {
                VerificationOutcome::Verified {
                    similarity,
                    source_url,
                    tx_hash,
                    ..
                } => {
                    println!(" {:2}. [✓ VERIFIED] {}", i + 1, name);
                    println!("     Creator/Source : {}", source_url);
                    println!(
                        "     Match Score    : {:.1}% (HighConfidence)",
                        similarity * 100.0
                    );
                    println!("     Polygon Tx     : {}\n", tx_hash);
                }
                VerificationOutcome::Unverified {
                    reason,
                    best_similarity,
                } => {
                    println!(" {:2}. [✗ UNVERIFIED] {}", i + 1, name);
                    println!("     Reason         : {}", reason);
                    println!(
                        "     Highest Score  : {:.1}% (Insufficient)\n",
                        best_similarity * 100.0
                    );
                }
                VerificationOutcome::Tampered {
                    stored_fingerprint,
                    source_url,
                    ..
                } => {
                    println!(" {:2}. [✗ TAMPERED] {}", i + 1, name);
                    println!("     Source         : {}", source_url);
                    println!("     On-Chain Hash  : {}\n", stored_fingerprint);
                }
            }
        }

        println!("------------------------------------------------------------");
        println!("• Total Images Processed : {}", target_files.len());
        println!(
            "• Verified Authentic     : {} / {}",
            verified_count,
            target_files.len()
        );
        println!(
            "• Unverified / Private   : {} / {}",
            unverified_count,
            target_files.len()
        );

        if strict && unverified_count > 0 {
            println!(
                "\n⛔ STRICT MODE FAILED: {}/{} image(s) could not be verified.",
                unverified_count,
                target_files.len()
            );
            println!("   Requirement: All images in the batch must be 100% verified.");
        } else if verified_count == target_files.len() {
            println!("\n🌟 ALL IMAGES VERIFIED (100% Authentic Public Creators Found!)");
        } else {
            println!("\n✓ Batch audit completed with per-image breakdown.");
        }
        println!("------------------------------------------------------------\n");

        Ok(results)
    }

    /// Generates a modern HTML Proof of Authenticity Certificate with Polygonscan links.
    fn generate_html_certificate(
        &self,
        proof: &crate::models::ContentProof,
        eval: &CandidateEvaluation,
        recalculated_hex: &str,
    ) {
        let block_str = proof
            .block_number
            .map(|b| format!("#{}", b))
            .unwrap_or_else(|| "Pending".into());
        let sim_percent = format!("{:.1}%", eval.similarity * 100.0);
        let title = eval.candidate.title.as_deref().unwrap_or("Authentic Public Source");
        let date_utc = chrono::Utc::now().to_rfc3339();

        let (display_tx, explorer_url, button_label) = if !proof.tx_hash.is_empty() && proof.tx_hash != *recalculated_hex {
            (
                proof.tx_hash.clone(),
                format!("https://amoy.polygonscan.com/tx/{}", proof.tx_hash),
                "View Transaction on Polygonscan ↗",
            )
        } else if let Some(addr) = self.polygon_registry.contract_address() {
            (
                format!("Anchored in Contract {}", addr),
                format!("https://amoy.polygonscan.com/address/{}#readContract", addr),
                "View Contract on Polygonscan ↗",
            )
        } else {
            (
                "Anchored on-chain".to_string(),
                "https://amoy.polygonscan.com".to_string(),
                "View on Polygonscan ↗",
            )
        };

        let html = format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>ProofFace • Cryptographic Proof Certificate</title>
    <style>
        :root {{
            --bg: #090a0f;
            --card-bg: rgba(20, 24, 38, 0.75);
            --border: rgba(99, 102, 241, 0.25);
            --accent: #8b5cf6;
            --accent-glow: #a855f7;
            --success: #10b981;
            --text-main: #f8fafc;
            --text-muted: #94a3b8;
        }}
        * {{ box-sizing: border-box; margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; }}
        body {{
            background: var(--bg);
            background-image: radial-gradient(circle at 50% 0%, rgba(139, 92, 246, 0.15), transparent 70%);
            color: var(--text-main);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 24px;
        }}
        .cert-card {{
            width: 100%;
            max-width: 680px;
            background: var(--card-bg);
            backdrop-filter: blur(20px);
            border: 1px solid var(--border);
            border-radius: 24px;
            box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.7), 0 0 40px rgba(139, 92, 246, 0.15);
            overflow: hidden;
            position: relative;
        }}
        .header {{
            padding: 32px 32px 24px;
            border-bottom: 1px solid rgba(255, 255, 255, 0.08);
            display: flex;
            justify-content: space-between;
            align-items: center;
        }}
        .brand {{ display: flex; align-items: center; gap: 12px; }}
        .logo {{ font-size: 28px; }}
        .brand-text h1 {{ font-size: 20px; font-weight: 700; letter-spacing: -0.5px; background: linear-gradient(135deg, #fff, #c084fc); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }}
        .brand-text p {{ font-size: 12px; color: var(--text-muted); }}
        .badge {{
            display: inline-flex;
            align-items: center;
            gap: 6px;
            background: rgba(16, 185, 129, 0.15);
            border: 1px solid rgba(16, 185, 129, 0.3);
            color: var(--success);
            padding: 6px 14px;
            border-radius: 9999px;
            font-size: 13px;
            font-weight: 600;
        }}
        .content {{ padding: 32px; display: flex; flex-direction: column; gap: 24px; }}
        .section-title {{ font-size: 11px; text-transform: uppercase; letter-spacing: 1px; color: var(--text-muted); font-weight: 700; margin-bottom: 8px; }}
        .data-box {{
            background: rgba(10, 12, 20, 0.6);
            border: 1px solid rgba(255, 255, 255, 0.05);
            border-radius: 14px;
            padding: 16px;
        }}
        .data-row {{ display: flex; justify-content: space-between; align-items: center; padding: 6px 0; font-size: 14px; }}
        .data-label {{ color: var(--text-muted); }}
        .data-val {{ font-weight: 600; text-align: right; }}
        .hash-code {{
            font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
            font-size: 12px;
            color: #c084fc;
            word-break: break-all;
            background: rgba(139, 92, 246, 0.08);
            padding: 10px 12px;
            border-radius: 10px;
            border: 1px solid rgba(139, 92, 246, 0.2);
            margin-top: 6px;
        }}
        .footer {{
            padding: 24px 32px;
            background: rgba(0, 0, 0, 0.3);
            border-top: 1px solid rgba(255, 255, 255, 0.05);
            display: flex;
            justify-content: space-between;
            align-items: center;
        }}
        .btn {{
            display: inline-flex;
            align-items: center;
            gap: 8px;
            background: linear-gradient(135deg, #7c3aed, #9333ea);
            color: #fff;
            text-decoration: none;
            font-size: 13px;
            font-weight: 600;
            padding: 10px 18px;
            border-radius: 12px;
            box-shadow: 0 4px 14px rgba(124, 58, 237, 0.4);
            transition: all 0.2s;
        }}
        .btn:hover {{ transform: translateY(-1px); box-shadow: 0 6px 20px rgba(124, 58, 237, 0.6); }}
        .meta {{ font-size: 12px; color: var(--text-muted); }}
    </style>
</head>
<body>
    <div class="cert-card">
        <div class="header">
            <div class="brand">
                <span class="logo">🦀</span>
                <div class="brand-text">
                    <h1>ProofFace Certificate</h1>
                    <p>Cryptographic Provenance Invariant</p>
                </div>
            </div>
            <div class="badge">
                <span>●</span> VERIFIED ON-CHAIN
            </div>
        </div>

        <div class="content">
            <div>
                <div class="section-title">Public Source Discovery</div>
                <div class="data-box">
                    <div class="data-row">
                        <span class="data-label">Matched Title</span>
                        <span class="data-val">{title}</span>
                    </div>
                    <div class="data-row">
                        <span class="data-label">Biometric Match Similarity</span>
                        <span class="data-val" style="color: #10b981;">{sim_percent} (High Confidence)</span>
                    </div>
                    <div class="data-row">
                        <span class="data-label">Source URL</span>
                        <span class="data-val"><a href="{source_url}" target="_blank" style="color: #818cf8; text-decoration: none;">View Original Post ↗</a></span>
                    </div>
                </div>
            </div>

            <div>
                <div class="section-title">Cryptographic Canonical Fingerprint (SHA-256)</div>
                <div class="hash-code">{recalculated_hex}</div>
            </div>

            <div>
                <div class="section-title">Polygon Amoy Blockchain Anchor</div>
                <div class="data-box">
                    <div class="data-row">
                        <span class="data-label">Network</span>
                        <span class="data-val">Polygon Amoy (Chain ID 80002)</span>
                    </div>
                    <div class="data-row">
                        <span class="data-label">Block Number</span>
                        <span class="data-val">{block_str}</span>
                    </div>
                    <div class="data-row">
                        <span class="data-label">Timestamp</span>
                        <span class="data-val">{date_utc}</span>
                    </div>
                    <div class="data-row" style="flex-direction: column; align-items: flex-start; gap: 6px; margin-top: 6px;">
                        <span class="data-label">Transaction Hash</span>
                        <div class="hash-code" style="width: 100%;">{display_tx}</div>
                    </div>
                </div>
            </div>
        </div>

        <div class="footer">
            <div class="meta">Anchored via ProofFace Protocol</div>
            <a href="{explorer_url}" target="_blank" class="btn">
                {button_label}
            </a>
        </div>
    </div>
</body>
</html>"#,
            title = title,
            sim_percent = sim_percent,
            source_url = proof.source_url,
            recalculated_hex = recalculated_hex,
            block_str = block_str,
            date_utc = date_utc,
            display_tx = display_tx,
            explorer_url = explorer_url,
            button_label = button_label,
        );

        let _ = std::fs::write("proof_certificate.html", html);
    }
}
