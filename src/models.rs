use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Bounding box representation for detected faces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoundingBox {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub confidence: f32,
}

/// A detected face with bounding box and image dimensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedFace {
    pub bbox: BoundingBox,
    pub image_width: u32,
    pub image_height: u32,
    pub face_crop_bytes: Vec<u8>,
}

/// Numerical face embedding vector (L2-normalized).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FaceEmbedding {
    pub values: Vec<f32>,
}

impl FaceEmbedding {
    pub fn new(values: Vec<f32>) -> Self {
        Self { values }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Request to search providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub max_results: usize,
    pub image_hint: Option<String>,
    pub image_bytes: Option<Vec<u8>>,
}

/// Individual search result returned by a search provider.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SearchResult {
    pub url: String,
    pub title: Option<String>,
    pub snippet: Option<String>,
    pub media_url: Option<String>,
    pub provider: String,
}

/// Discovered public content subject to canonicalization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredContent {
    pub source_url: String,
    pub media_url: Option<String>,
    pub title: Option<String>,
    pub snippet: Option<String>,
    pub content_hash: String,
    pub retrieved_at: DateTime<Utc>,
}

/// Candidate pair for verification.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub source_url: String,
    pub media_url: String,
    pub title: Option<String>,
    pub snippet: Option<String>,
    pub raw_image_bytes: Vec<u8>,
}

/// Similarity level classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchConfidence {
    HighConfidence,
    PossibleMatch,
    NoMatch,
}

/// Result of evaluating a single candidate.
#[derive(Debug, Clone)]
pub struct CandidateEvaluation {
    pub candidate: Candidate,
    pub similarity: f32,
    pub match_confidence: MatchConfidence,
}

/// On-chain proof representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofRecord {
    pub fingerprint_hex: String,
    #[serde(skip)]
    pub fingerprint_bytes: [u8; 32],
    pub source_url: String,
    pub tx_hash: String,
    pub block_number: Option<u64>,
    pub timestamp: u64,
}

pub type ContentProof = ProofRecord;

/// Final outcome of verification pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationOutcome {
    Verified {
        fingerprint: String,
        tx_hash: String,
        source_url: String,
        similarity: f32,
    },
    Tampered {
        stored_fingerprint: String,
        recalculated_fingerprint: String,
        source_url: String,
    },
    Unverified {
        reason: String,
        best_similarity: f32,
    },
}
