use crate::error::{PipelineError, Result};
use crate::models::{FaceEmbedding, MatchConfidence};

/// Computes the cosine similarity between two L2-normalized embeddings.
/// Returns a value between -1.0 and 1.0.
pub fn cosine_similarity(a: &FaceEmbedding, b: &FaceEmbedding) -> Result<f32> {
    if a.len() != b.len() {
        return Err(PipelineError::DimensionMismatch {
            expected: a.len(),
            got: b.len(),
        });
    }

    if a.is_empty() {
        return Err(PipelineError::FaceModelError(
            "Cannot calculate similarity on empty embedding".into(),
        ));
    }

    let mut dot_product = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for (x, y) in a.values.iter().zip(b.values.iter()) {
        dot_product += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let denominator = (norm_a.sqrt() * norm_b.sqrt()).max(f32::EPSILON);
    let similarity = dot_product / denominator;

    // Clamp to [-1.0, 1.0] to guard against floating-point imprecision
    Ok(similarity.clamp(-1.0, 1.0))
}

/// Classifies match confidence based on configured thresholds.
pub fn evaluate_similarity(
    similarity: f32,
    high_threshold: f32,
    possible_threshold: f32,
) -> MatchConfidence {
    if similarity >= high_threshold {
        MatchConfidence::HighConfidence
    } else if similarity >= possible_threshold {
        MatchConfidence::PossibleMatch
    } else {
        MatchConfidence::NoMatch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_vectors_have_similarity_one() {
        let v1 = FaceEmbedding::new(vec![0.5, 0.5, 0.5, 0.5]);
        let v2 = FaceEmbedding::new(vec![0.5, 0.5, 0.5, 0.5]);
        let sim = cosine_similarity(&v1, &v2).unwrap();
        assert!((sim - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_orthogonal_vectors_have_similarity_zero() {
        let v1 = FaceEmbedding::new(vec![1.0, 0.0, 0.0]);
        let v2 = FaceEmbedding::new(vec![0.0, 1.0, 0.0]);
        let sim = cosine_similarity(&v1, &v2).unwrap();
        assert!(sim.abs() < 1e-5);
    }

    #[test]
    fn test_opposite_vectors_have_similarity_minus_one() {
        let v1 = FaceEmbedding::new(vec![1.0, 0.0]);
        let v2 = FaceEmbedding::new(vec![-1.0, 0.0]);
        let sim = cosine_similarity(&v1, &v2).unwrap();
        assert!((sim - (-1.0)).abs() < 1e-5);
    }

    #[test]
    fn test_dimension_mismatch_returns_error() {
        let v1 = FaceEmbedding::new(vec![1.0, 0.0]);
        let v2 = FaceEmbedding::new(vec![1.0, 0.0, 0.0]);
        let result = cosine_similarity(&v1, &v2);
        assert!(result.is_err());
        match result.unwrap_err() {
            PipelineError::DimensionMismatch { expected, got } => {
                assert_eq!(expected, 2);
                assert_eq!(got, 3);
            }
            _ => panic!("Expected DimensionMismatch error"),
        }
    }

    #[test]
    fn test_empty_vectors_returns_error() {
        let v1 = FaceEmbedding::new(vec![]);
        let v2 = FaceEmbedding::new(vec![]);
        assert!(cosine_similarity(&v1, &v2).is_err());
    }

    #[test]
    fn test_evaluate_confidence() {
        assert_eq!(
            evaluate_similarity(0.85, 0.75, 0.55),
            MatchConfidence::HighConfidence
        );
        assert_eq!(
            evaluate_similarity(0.65, 0.75, 0.55),
            MatchConfidence::PossibleMatch
        );
        assert_eq!(
            evaluate_similarity(0.40, 0.75, 0.55),
            MatchConfidence::NoMatch
        );
    }
}
