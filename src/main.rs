use clap::Parser;
use proofface::blockchain::polygon::PolygonRegistry;
use proofface::cli::{Cli, Commands};
use proofface::config::Config;
use proofface::error::Result;
use proofface::pipeline::Pipeline;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize structured tracing
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .compact(),
        )
        .init();

    let config = Config::from_env()?;

    match cli.command {
        Commands::Verify {
            image_paths,
            query,
            strict,
        } => {
            let pipeline = Pipeline::new(config);
            if image_paths.len() == 1 && image_paths[0].is_file() {
                match pipeline
                    .run_verification(&image_paths[0], query.as_deref())
                    .await
                {
                    Ok(_) => std::process::exit(0),
                    Err(e) => {
                        eprintln!("\nPipeline execution halted: {e}");
                        std::process::exit(1);
                    }
                }
            } else {
                match pipeline.run_batch_verification(&image_paths, strict).await {
                    Ok(results) => {
                        if strict
                            && results.iter().any(|(_, r)| {
                                !matches!(
                                    r,
                                    proofface::models::VerificationOutcome::Verified { .. }
                                )
                            })
                        {
                            std::process::exit(1);
                        } else {
                            std::process::exit(0);
                        }
                    }
                    Err(e) => {
                        eprintln!("\nBatch verification failed: {e}");
                        std::process::exit(1);
                    }
                }
            }
        }
        Commands::Batch {
            image_paths,
            strict,
        } => {
            let pipeline = Pipeline::new(config);
            match pipeline.run_batch_verification(&image_paths, strict).await {
                Ok(results) => {
                    if strict
                        && results.iter().any(|(_, r)| {
                            !matches!(r, proofface::models::VerificationOutcome::Verified { .. })
                        })
                    {
                        std::process::exit(1);
                    } else {
                        std::process::exit(0);
                    }
                }
                Err(e) => {
                    eprintln!("\nBatch verification failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        Commands::TamperDemo { image_path, query } => {
            let pipeline = Pipeline::new(config);
            match pipeline
                .run_tamper_demo(&image_path, query.as_deref())
                .await
            {
                Ok(_) => std::process::exit(0),
                Err(e) => {
                    eprintln!("\nTamper demo halted: {e}");
                    std::process::exit(1);
                }
            }
        }
        Commands::InspectProof { fingerprint } => {
            println!("Inspecting on-chain proof for fingerprint: {}", fingerprint);
            let registry = PolygonRegistry::new(
                config.rpc_primary.clone(),
                config.rpc_secondary.clone(),
                config.contract_address.clone(),
                config.wallet_private_key.clone(),
            );

            let clean_hex = fingerprint.trim_start_matches("0x");
            let mut fp_bytes = [0u8; 32];
            if let Ok(bytes) = hex::decode(clean_hex) {
                if bytes.len() == 32 {
                    fp_bytes.copy_from_slice(&bytes);
                    match registry.get_on_chain_proof(&fp_bytes).await {
                        Ok(Some(proof)) => {
                            println!("\n✓ Proof Found On-Chain:");
                            println!("  Fingerprint: {}", proof.fingerprint_hex);
                            println!("  Source URL : {}", proof.source_url);
                            println!("  Timestamp  : {}", proof.timestamp);
                        }
                        Ok(None) => {
                            println!("\n✗ No on-chain proof found for given fingerprint.");
                        }
                        Err(e) => {
                            eprintln!("\nRPC query failed: {e}");
                        }
                    }
                } else {
                    eprintln!("Error: Fingerprint must be 32 bytes (64 hex characters).");
                }
            } else {
                eprintln!("Error: Invalid hexadecimal string.");
            }
        }
        Commands::Health => {
            println!("╔══════════════════════════════════════════════════════════╗");
            println!("║             PROOFFACE 🦀 HEALTH CHECK                    ║");
            println!("╚══════════════════════════════════════════════════════════╝\n");
            println!("• Search Provider         : {}", config.search_provider);
            println!(
                "• Search Fallback Provider: {:?}",
                config.search_fallback_provider
            );
            println!("• Polygon RPC Primary     : {}", config.rpc_primary);
            println!("• Polygon RPC Secondary   : {:?}", config.rpc_secondary);
            println!("• Chain ID                : {}", config.chain_id);
            println!(
                "• Match Thresholds        : High >= {:.2}, Possible >= {:.2}",
                config.high_confidence_threshold, config.possible_match_threshold
            );
            println!(
                "• Max Concurrency         : {}",
                config.max_concurrent_candidates
            );
            println!("\nConfiguration valid and ready.");
        }
    }

    Ok(())
}
