use crate::error::{PipelineError, Result};
use crate::models::{DetectedFace, FaceEmbedding};
use image::imageops::FilterType;

pub const EMBEDDING_DIM: usize = 128;
pub const TARGET_FACE_DIM: u32 = 112;

pub struct FaceEmbedder;

impl FaceEmbedder {
    pub fn new() -> Self {
        Self
    }

    /// Computes a normalized 128-dimensional embedding from a detected face crop.
    pub fn generate_embedding(&self, face: &DetectedFace) -> Result<FaceEmbedding> {
        let img = image::load_from_memory(&face.face_crop_bytes).map_err(|e| {
            PipelineError::FaceModelError(format!("Failed to load face crop bytes: {e}"))
        })?;

        // Resize face crop to standard 112x112
        let resized = img.resize_exact(TARGET_FACE_DIM, TARGET_FACE_DIM, FilterType::Triangle);
        let gray = resized.to_luma8();

        let mut raw_vector = vec![0.0f32; EMBEDDING_DIM];

        // Spatial grid cell feature pooling (8x8 grid -> 64 spatial cells x 2 (intensity + gradient)) = 128 dimensions
        let cell_size = TARGET_FACE_DIM / 8; // 14x14 pixels per cell

        for grid_y in 0..8 {
            for grid_x in 0..8 {
                let cell_idx = (grid_y * 8 + grid_x) as usize;

                let start_x = grid_x * cell_size;
                let start_y = grid_y * cell_size;

                let mut cell_sum = 0.0f32;
                let mut cell_grad = 0.0f32;
                let mut count = 0.0f32;

                for cy in 0..cell_size {
                    for cx in 0..cell_size {
                        let px = (start_x + cx).min(TARGET_FACE_DIM - 1);
                        let py = (start_y + cy).min(TARGET_FACE_DIM - 1);

                        let val = gray.get_pixel(px, py)[0] as f32 / 255.0;
                        cell_sum += val;

                        // Local gradient approximation
                        let right = if px + 1 < TARGET_FACE_DIM {
                            gray.get_pixel(px + 1, py)[0] as f32 / 255.0
                        } else {
                            val
                        };

                        let down = if py + 1 < TARGET_FACE_DIM {
                            gray.get_pixel(px, py + 1)[0] as f32 / 255.0
                        } else {
                            val
                        };

                        cell_grad += ((right - val).powi(2) + (down - val).powi(2)).sqrt();
                        count += 1.0;
                    }
                }

                let mean_intensity = cell_sum / count.max(1.0);
                let mean_grad = cell_grad / count.max(1.0);

                raw_vector[cell_idx] = mean_intensity;
                raw_vector[64 + cell_idx] = mean_grad;
            }
        }

        // L2 Normalization
        let norm_sq: f32 = raw_vector.iter().map(|v| v * v).sum();
        let norm = norm_sq.sqrt().max(f32::EPSILON);

        let normalized_vector: Vec<f32> = raw_vector.iter().map(|v| v / norm).collect();

        Ok(FaceEmbedding::new(normalized_vector))
    }
}
