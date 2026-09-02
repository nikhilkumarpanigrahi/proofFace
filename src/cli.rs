use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "proofface",
    version = "0.1.0",
    about = "ProofFace 🦀 Face -> Web Discovery -> Blockchain Proof Verification Pipeline",
    long_about = "ProofFace takes face images, discovers public matching web sources, verifies candidate similarity, creates a deterministic SHA-256 fingerprint, and anchors/verifies proofs on Polygon Amoy testnet."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose structured debug tracing
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run verification pipeline on single or multiple face images
    Verify {
        /// Path(s) to input JPEG/PNG images or directory containing images
        #[arg(value_name = "IMAGE_PATHS", required = true)]
        image_paths: Vec<PathBuf>,

        /// Optional custom search query or person/influencer name (single image mode)
        #[arg(short, long, value_name = "SEARCH_QUERY")]
        query: Option<String>,

        /// Require 100% of images to be verified in multi-image mode (fails if any image is unverified)
        #[arg(short, long)]
        strict: bool,
    },

    /// Run batch verification across multiple images or a directory (alias for multi-image verify)
    Batch {
        /// List of image paths or directory containing images
        #[arg(value_name = "IMAGE_PATHS", required = true)]
        image_paths: Vec<PathBuf>,

        /// Require 100% of images to be verified (fails if any image is unverified)
        #[arg(short, long)]
        strict: bool,
    },

    /// Demonstrate successful verification followed by cryptographic tamper detection
    TamperDemo {
        /// Path to input JPEG or PNG image
        #[arg(value_name = "IMAGE_PATH")]
        image_path: PathBuf,

        /// Optional custom search query or person/influencer name
        #[arg(short, long, value_name = "SEARCH_QUERY")]
        query: Option<String>,
    },

    /// Inspect a recorded proof directly from Polygon Amoy testnet
    InspectProof {
        /// 0x-prefixed 32-byte hexadecimal SHA-256 fingerprint
        #[arg(value_name = "FINGERPRINT")]
        fingerprint: String,
    },

    /// Health check for configured search providers and Polygon RPC endpoints
    Health,
}
