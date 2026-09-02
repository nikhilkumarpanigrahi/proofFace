use thiserror::Error;

#[derive(Error, Debug)]
pub enum PipelineError {
    #[error("Invalid image input: {0}")]
    InvalidImage(String),

    #[error("No face detected in input image")]
    NoFaceDetected,

    #[error("Ambiguous face input: found {0} faces, expected exactly 1")]
    AmbiguousFaceInput(usize),

    #[error("Face model processing failed: {0}")]
    FaceModelError(String),

    #[error("Vector dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    #[error("Search provider error ({provider}): {message}")]
    SearchProviderError { provider: String, message: String },

    #[error("All search providers failed (exhausted fallback pool)")]
    SearchExhausted,

    #[error("Search timeout after {0}ms")]
    SearchTimeout(u64),

    #[error("Candidate fetch failed for {url}: {reason}")]
    CandidateFetchFailed { url: String, reason: String },

    #[error("No matching candidate found above threshold (best similarity: {best_similarity:.2})")]
    NoMatchFound { best_similarity: f32 },

    #[error("Canonicalization failed: {0}")]
    CanonicalizationFailed(String),

    #[error("Crypto / Hashing failed: {0}")]
    CryptoError(String),

    #[error("Blockchain RPC error ({endpoint}): {message}")]
    BlockchainRpcError { endpoint: String, message: String },

    #[error("Blockchain transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Blockchain confirmation timeout")]
    ConfirmationTimeout,

    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, PipelineError>;
