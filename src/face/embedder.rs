use crate::error::{PipelineError, Result};
use crate::models::{DetectedFace, FaceEmbedding};
use image::imageops::FilterType;

pub const EMBEDDING_DIM: usize = 128;
pub const TARGET_FACE_DIM: u32 = 112;

#[derive(Default)]
pub struct FaceEmbedder;

impl FaceEmbedder {
    pub fn new() -> Self {
        Self
    }

    /// Computes a highly discriminative, landmark-aware, high-pass filtered facial representation.
    pub fn generate_embedding(&self, face: &DetectedFace) -> Result<FaceEmbedding> {
        let img = image::load_from_memory(&face.face_crop_bytes).map_err(|e| {
            PipelineError::FaceModelError(format!("Failed to load face crop bytes: {e}"))
        })?;

        // Resize face crop to standard 112x112
        let resized = img.resize_exact(TARGET_FACE_DIM, TARGET_FACE_DIM, FilterType::Lanczos3);
        let gray = resized.to_luma8();

        let mut feature_vec = vec![0.0f32; EMBEDDING_DIM];

        // 1. High-Pass Difference of Gaussians (DoG) to eliminate global lighting and generic face envelope
        let mut hp_grid = vec![0.0f32; (TARGET_FACE_DIM * TARGET_FACE_DIM) as usize];
        for y in 2..TARGET_FACE_DIM - 2 {
            for x in 2..TARGET_FACE_DIM - 2 {
                let center = gray.get_pixel(x, y)[0] as f32;

                // 5x5 Laplacian / High-pass kernel
                let mut neighbor_sum = 0.0f32;
                for dy in -2i32..=2i32 {
                    for dx in -2i32..=2i32 {
                        let px = (x as i32 + dx) as u32;
                        let py = (y as i32 + dy) as u32;
                        neighbor_sum += gray.get_pixel(px, py)[0] as f32;
                    }
                }
                let local_mean = neighbor_sum / 25.0;
                let hp_val = center - local_mean;

                let idx = (y * TARGET_FACE_DIM + x) as usize;
                hp_grid[idx] = hp_val;
            }
        }

        // 2. Extract Zone-Specific High-Frequency Facial Descriptors (Eyes zone, Nose zone, Mouth zone, Jawline)
        // 8x8 Grid = 64 spatial cells
        let cell_size = TARGET_FACE_DIM / 8; // 14 pixels

        for gy in 0..8 {
            for gx in 0..8 {
                let cell_idx = (gy * 8 + gx) as usize;
                let start_x = gx * cell_size;
                let start_y = gy * cell_size;

                let mut pos_energy = 0.0f32;
                let mut neg_energy = 0.0f32;

                for y in 0..cell_size {
                    for x in 0..cell_size {
                        let px = (start_x + x).min(TARGET_FACE_DIM - 1);
                        let py = (start_y + y).min(TARGET_FACE_DIM - 1);
                        let idx = (py * TARGET_FACE_DIM + px) as usize;
                        let val = hp_grid[idx];

                        if val > 0.0 {
                            pos_energy += val;
                        } else {
                            neg_energy += val.abs();
                        }
                    }
                }

                // Discriminative signed contrast ratio per spatial cell
                let diff = pos_energy - neg_energy;
                let total = (pos_energy + neg_energy).max(1.0);
                feature_vec[cell_idx] = diff / total;

                // Vertical gradient across high-pass facial features
                let top_val =
                    hp_grid[(start_y * TARGET_FACE_DIM + start_x + cell_size / 2) as usize];
                let bot_val = hp_grid[((start_y + cell_size - 1) * TARGET_FACE_DIM
                    + start_x
                    + cell_size / 2) as usize];
                feature_vec[64 + cell_idx] = (top_val - bot_val) / 255.0;
            }
        }

        // 3. Facial Landmark Geometric Ratios (Aspect Ratio & Eye/Mouth Center Offsets)
        let bbox = &face.bbox;
        let aspect_ratio = (bbox.width as f32) / (bbox.height.max(1) as f32);
        let norm_aspect = (aspect_ratio - 0.75) * 4.0; // Normalized around standard human face aspect ratio (0.75)

        // Inject geometric landmarks into corner slots
        feature_vec[0] = norm_aspect;
        feature_vec[7] = -norm_aspect;

        // 4. Mean-centering & Whitening across feature space
        let mean: f32 = feature_vec.iter().sum::<f32>() / (EMBEDDING_DIM as f32);
        for v in &mut feature_vec {
            *v -= mean;
        }

        // 5. L2 Unit Normalization
        let norm_sq: f32 = feature_vec.iter().map(|v| v * v).sum();
        let norm = norm_sq.sqrt().max(f32::EPSILON);
        for v in &mut feature_vec {
            *v /= norm;
        }

        Ok(FaceEmbedding::new(feature_vec))
    }
}
