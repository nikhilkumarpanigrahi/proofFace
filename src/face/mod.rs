pub mod detector;
pub mod embedder;
pub mod similarity;

use crate::error::{PipelineError, Result};
use crate::models::{DetectedFace, FaceEmbedding};
use async_trait::async_trait;
use detector::FaceDetector;
use embedder::FaceEmbedder;

#[async_trait]
pub trait FaceEngine: Send + Sync {
    async fn detect(&self, image_bytes: &[u8]) -> Result<Vec<DetectedFace>>;
    async fn embed(&self, face: &DetectedFace) -> Result<FaceEmbedding>;
}

pub struct DefaultFaceEngine {
    detector: FaceDetector,
    embedder: FaceEmbedder,
}

impl DefaultFaceEngine {
    pub fn new() -> Self {
        Self {
            detector: FaceDetector::new(),
            embedder: FaceEmbedder::new(),
        }
    }
}

impl Default for DefaultFaceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FaceEngine for DefaultFaceEngine {
    async fn detect(&self, image_bytes: &[u8]) -> Result<Vec<DetectedFace>> {
        let img = self.detector.validate_and_load(image_bytes)?;
        let faces = self.detector.detect_faces(&img)?;

        if faces.is_empty() {
            return Err(PipelineError::NoFaceDetected);
        }

        Ok(faces)
    }

    async fn embed(&self, face: &DetectedFace) -> Result<FaceEmbedding> {
        self.embedder.generate_embedding(face)
    }
}
