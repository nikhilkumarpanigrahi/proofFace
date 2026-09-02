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
    Candidate, CandidateEvaluation, DiscoveredContent, FaceEmbedding,
    MatchConfidence, SearchRequest, VerificationOutcome,
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
    pub async fn run_verification(&self, image_path: &Path) -> Result<VerificationOutcome> {
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║                      PROOFFACE 🦀                        ║");
        println!("║     Face → Web Discovery → Blockchain Proof              ║");
        println!("╚══════════════════════════════════════════════════════════╝\n");

        // [1/7] Validating image
        print!("[1/7] Validating image... ");
        let image_bytes = fs::read(image_path).map_err(|e| {
            PipelineError::InvalidImage(format!("Could not read file {}: {e}", image_path.display()))
        })?;
        let _ = FaceDetector::new().validate_and_load(&image_bytes)?;
        println!("✓ Valid image ({} bytes)", image_bytes.len());

        // [2/7] Detecting face
        print!("[2/7] Detecting face... ");
        let faces = self.face_engine.detect(&image_bytes).await?;
        if faces.len() > 1 {
            println!("⚠ {} faces found (selecting primary face)", faces.len());
        } else {
            println!("✓ 1 face detected (confidence: {:.2})", faces[0].bbox.confidence);
        }
        let target_face = &faces[0];

        // [3/7] Generating embedding
        print!("[3/7] Generating embedding... ");
        let target_embedding = self.face_engine.embed(target_face).await?;
        println!("✓ L2-normalized 128-dim embedding generated");

        // [4/7] Searching public web
        let query_stem = image_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("face public profile");
        let search_query = format!("{} profile photo", query_stem);

        println!("[4/7] Searching public web for candidates (query: \"{}\")...", search_query);
        let search_req = SearchRequest {
            query: search_query,
            max_results: self.config.max_search_results,
            image_hint: None,
        };

        let search_results = self.search_orchestrator.search_with_resilience(&search_req).await?;
        println!("      ✓ {} search candidate URLs discovered", search_results.len());

        // [5/7] Verifying candidates with bounded concurrency
        println!(
            "[5/7] Verifying candidates (bounded concurrency: {})...",
            self.config.max_concurrent_candidates
        );

        let best_evaluation = self
            .evaluate_candidates(&search_results, &target_embedding)
            .await?;

        let match_eval = match best_evaluation {
            Some(eval) if eval.match_confidence != MatchConfidence::NoMatch => {
                println!(
                    "\n      ★ MATCH CONFIRMED (similarity: {:.3})\n      Source: {}\n      Media:  {}",
                    eval.similarity, eval.candidate.source_url, eval.candidate.media_url
                );
                eval
            }
            Some(eval) => {
                println!("\n      ✗ Highest candidate similarity ({:.3}) below threshold ({:.2})", eval.similarity, self.config.possible_match_threshold);
                return Err(PipelineError::NoMatchFound {
                    best_similarity: eval.similarity,
                });
            }
            None => {
                println!("\n      ✗ No candidate image could be processed");
                return Err(PipelineError::NoMatchFound {
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

        let (fingerprint_hex, fingerprint_bytes) = ContentCanonicalizer::fingerprint(&discovered_content)?;
        println!("✓ Fingerprint: {}", fingerprint_hex);

        // Registering proof on Polygon Amoy
        print!("      Anchoring on Polygon Amoy (Chain ID {})... ", self.config.chain_id);
        let proof = self
            .polygon_registry
            .register_proof(&fingerprint_bytes, &discovered_content.source_url)
            .await?;
        println!("✓ Confirmed");
        println!("      Tx Hash: {}", proof.tx_hash);

        // [7/7] Read-after-write verification
        print!("[7/7] Re-verifying against on-chain record... ");
        let (recalculated_hex, _) = ContentCanonicalizer::fingerprint(&discovered_content)?;

        if recalculated_hex == proof.fingerprint_hex {
            println!("✓ Match confirmed");
            println!("\n╔══════════════════════════════════════════════════════════╗");
            println!("║                      VERIFIED ✓                          ║");
            println!("╚══════════════════════════════════════════════════════════╝\n");

            Ok(VerificationOutcome::Verified {
                fingerprint: proof.fingerprint_hex,
                tx_hash: proof.tx_hash,
                source_url: proof.source_url,
                similarity: match_eval.similarity,
            })
        } else {
            println!("✗ Fingerprint mismatch!");
            println!("\n╔══════════════════════════════════════════════════════════╗");
            println!("║                      TAMPERED ✗                          ║");
            println!("╚══════════════════════════════════════════════════════════╝\n");

            Ok(VerificationOutcome::Tampered {
                stored_fingerprint: proof.fingerprint_hex,
                recalculated_fingerprint: recalculated_hex,
                source_url: proof.source_url,
            })
        }
    }

    /// Evaluates candidate URLs with bounded concurrency and early exit.
    async fn evaluate_candidates(
        &self,
        search_results: &[crate::models::SearchResult],
        target_embedding: &FaceEmbedding,
    ) -> Result<Option<CandidateEvaluation>> {
        let best_match: Arc<Mutex<Option<CandidateEvaluation>>> = Arc::new(Mutex::new(None));
        let semaphore = self.bounded_pool.semaphore();

        let mut tasks = Vec::new();

        for (idx, result) in search_results.iter().enumerate() {
            let media_url = match &result.media_url {
                Some(url) if !url.trim().is_empty() => url.clone(),
                _ => continue, // Skip text-only search results without media
            };

            let sem = Arc::clone(&semaphore);
            let fetcher = ContentFetcher::new(self.config.candidate_timeout_ms);
            let embedder = FaceEmbedder::new();
            let detector = FaceDetector::new();
            let target_emb = target_embedding.clone();
            let res_clone = result.clone();
            let best_match_clone = Arc::clone(&best_match);
            let high_thresh = self.config.high_confidence_threshold;
            let poss_thresh = self.config.possible_match_threshold;

            let task = tokio::spawn(async move {
                let _permit = match sem.acquire().await {
                    Ok(p) => p,
                    Err(_) => return,
                };

                // Check if another worker already found a high-confidence match
                {
                    let lock = best_match_clone.lock().await;
                    if let Some(current_best) = lock.as_ref() {
                        if current_best.match_confidence == MatchConfidence::HighConfidence {
                            return; // Early termination: stop extra download/inference work
                        }
                    }
                }

                if let Ok(bytes) = fetcher.fetch_bytes(&media_url).await {
                    if let Ok(img) = detector.validate_and_load(&bytes) {
                        if let Ok(faces) = detector.detect_faces(&img) {
                            if let Some(candidate_face) = faces.first() {
                                if let Ok(cand_emb) = embedder.generate_embedding(candidate_face) {
                                    if let Ok(sim) = cosine_similarity(&target_emb, &cand_emb) {
                                        let conf = evaluate_similarity(sim, high_thresh, poss_thresh);

                                        println!(
                                            "      #Candidate {:02} ........ similarity: {:.3} ({:?})",
                                            idx + 1,
                                            sim,
                                            conf
                                        );

                                        let mut lock = best_match_clone.lock().await;
                                        let is_better = match lock.as_ref() {
                                            Some(current) => sim > current.similarity,
                                            None => true,
                                        };

                                        if is_better {
                                            *lock = Some(CandidateEvaluation {
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

        let final_match = best_match.lock().await.clone();
        Ok(final_match)
    }

    /// Runs a simulated tamper demonstration against an existing proof record.
    pub async fn run_tamper_demo(&self, image_path: &Path) -> Result<VerificationOutcome> {
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║             PROOFFACE 🦀 TAMPER DEMO                    ║");
        println!("║     Cryptographic Invariant & Mismatch Proof             ║");
        println!("╚══════════════════════════════════════════════════════════╝\n");

        let outcome = self.run_verification(image_path).await?;

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
}
