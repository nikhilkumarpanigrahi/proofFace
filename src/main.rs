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
                config.chain_id,
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
        Commands::DeployContract { private_key } => {
            println!("╔══════════════════════════════════════════════════════════╗");
            println!("║      PROOFFACE 🦀 CONTENTREGISTRY CONTRACT DEPLOYER      ║");
            println!("║         Target Network: Polygon Amoy (Chain ID 80002)    ║");
            println!("╚══════════════════════════════════════════════════════════╝\n");

            let pk = private_key
                .or_else(|| config.wallet_private_key.clone())
                .filter(|s| !s.trim().is_empty());

            let pk_str = match pk {
                Some(k) => k,
                None => {
                    eprintln!("Error: No wallet private key provided!");
                    eprintln!("Please either:");
                    eprintln!("  1. Pass key via flag: cargo run -- deploy-contract --private-key 0x...");
                    eprintln!("  2. Or add WALLET_PRIVATE_KEY=0x... in your .env file.");
                    eprintln!("\nFree Polygon Amoy testnet POL faucet: https://faucet.polygon.technology/");
                    std::process::exit(1);
                }
            };

            let registry = PolygonRegistry::new(
                config.rpc_primary.clone(),
                config.rpc_secondary.clone(),
                None,
                Some(pk_str.clone()),
                config.chain_id,
            );

            let deployer = proofface::blockchain::deployer::ContractDeployer::new(&registry);
            match deployer.deploy(&pk_str).await {
                Ok(contract_address) => {
                    println!("\n╔══════════════════════════════════════════════════════════╗");
                    println!("║             DEPLOYMENT SUCCESSFUL! ✓                     ║");
                    println!("╚══════════════════════════════════════════════════════════╝");
                    println!("Contract Address: {}", contract_address);
                    println!("Polygonscan Link: https://amoy.polygonscan.com/address/{}", contract_address);
                    println!("\nℹ Action Required:");
                    println!("Add this to your .env file:");
                    println!("CONTRACT_ADDRESS={}", contract_address);

                    if let Ok(env_content) = std::fs::read_to_string(".env") {
                        let updated = if env_content.contains("CONTRACT_ADDRESS=") {
                            let lines: Vec<String> = env_content
                                .lines()
                                .map(|l| {
                                    if l.starts_with("CONTRACT_ADDRESS=") {
                                        format!("CONTRACT_ADDRESS={}", contract_address)
                                    } else {
                                        l.to_string()
                                    }
                                })
                                .collect();
                            lines.join("\n")
                        } else {
                            format!("{}\nCONTRACT_ADDRESS={}\n", env_content.trim(), contract_address)
                        };
                        let _ = std::fs::write(".env", updated);
                        println!("✓ Automatically updated .env with CONTRACT_ADDRESS");
                    }
                }
                Err(e) => {
                    eprintln!("\nContract deployment failed: {e}");
                    std::process::exit(1);
                }
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
