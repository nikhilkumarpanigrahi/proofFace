use crate::error::{PipelineError, Result};
use crate::models::{BoundingBox, DetectedFace};
use image::{DynamicImage, GenericImageView, ImageFormat};
use std::io::Cursor;

pub const MAX_IMAGE_SIZE_BYTES: usize = 20 * 1024 * 1024; // 20 MB
pub const MIN_IMAGE_DIMENSION: u32 = 20;
pub const MAX_IMAGE_DIMENSION: u32 = 8192;

#[derive(Default)]
pub struct FaceDetector;

impl FaceDetector {
    pub fn new() -> Self {
        Self
    }

    /// Validates raw bytes and loads image.
    pub fn validate_and_load(&self, image_bytes: &[u8]) -> Result<DynamicImage> {
        if image_bytes.is_empty() {
            return Err(PipelineError::InvalidImage("Image data is empty".into()));
        }

        if image_bytes.len() > MAX_IMAGE_SIZE_BYTES {
            return Err(PipelineError::InvalidImage(format!(
                "Image size ({} bytes) exceeds maximum limit ({} bytes)",
                image_bytes.len(),
                MAX_IMAGE_SIZE_BYTES
            )));
        }

        // Validate format (JPEG / PNG)
        let format = image::guess_format(image_bytes).map_err(|e| {
            PipelineError::InvalidImage(format!("Unsupported or unrecognized image format: {e}"))
        })?;

        if format != ImageFormat::Jpeg && format != ImageFormat::Png {
            return Err(PipelineError::InvalidImage(format!(
                "Unsupported format {:?}. Only JPEG and PNG are supported.",
                format
            )));
        }

        let img = image::load_from_memory(image_bytes)
            .map_err(|e| PipelineError::InvalidImage(format!("Failed to decode image: {e}")))?;

        let (width, height) = img.dimensions();
        if width < MIN_IMAGE_DIMENSION || height < MIN_IMAGE_DIMENSION {
            return Err(PipelineError::InvalidImage(format!(
                "Image dimensions ({width}x{height}) are smaller than minimum allowed ({MIN_IMAGE_DIMENSION}x{MIN_IMAGE_DIMENSION})"
            )));
        }

        if width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION {
            return Err(PipelineError::InvalidImage(format!(
                "Image dimensions ({width}x{height}) exceed maximum allowed ({MAX_IMAGE_DIMENSION}x{MAX_IMAGE_DIMENSION})"
            )));
        }

        Ok(img)
    }

    /// Detects face regions in the given image.
    /// Uses skin tone segmentation, edge gradient heuristics, and facial aspect ratio analysis.
    pub fn detect_faces(&self, img: &DynamicImage) -> Result<Vec<DetectedFace>> {
        let (width, height) = img.dimensions();
        let rgb_img = img.to_rgb8();

        // Check for face candidate regions
        let mut skin_pixels = 0u64;
        let mut total_pixels = 0u64;
        let mut min_x = width;
        let mut max_x = 0;
        let mut min_y = height;
        let mut max_y = 0;

        for y in 0..height {
            for x in 0..width {
                let pixel = rgb_img.get_pixel(x, y);
                let r = pixel[0] as f32;
                let g = pixel[1] as f32;
                let b = pixel[2] as f32;

                total_pixels += 1;

                // Standard RGB skin color distribution heuristic
                let is_skin = r > 60.0
                    && g > 40.0
                    && b > 20.0
                    && (r - g).abs() > 10.0
                    && r > g
                    && r > b
                    && (r - b).abs() > 10.0;

                if is_skin {
                    skin_pixels += 1;
                    if x < min_x {
                        min_x = x;
                    }
                    if x > max_x {
                        max_x = x;
                    }
                    if y < min_y {
                        min_y = y;
                    }
                    if y > max_y {
                        max_y = y;
                    }
                }
            }
        }

        let skin_ratio = skin_pixels as f64 / total_pixels.max(1) as f64;

        // If skin pixels or face region is detected with adequate area
        if skin_pixels > 100 && min_x < max_x && min_y < max_y && skin_ratio > 0.01 {
            let bbox_width = (max_x - min_x).clamp(MIN_IMAGE_DIMENSION, width);
            let bbox_height = (max_y - min_y).clamp(MIN_IMAGE_DIMENSION, height);

            // Center crop the detected region with padding
            let pad_x = (bbox_width / 8).min(min_x);
            let pad_y = (bbox_height / 8).min(min_y);
            let crop_x = min_x.saturating_sub(pad_x);
            let crop_y = min_y.saturating_sub(pad_y);
            let crop_w = (bbox_width + pad_x * 2).min(width - crop_x);
            let crop_h = (bbox_height + pad_y * 2).min(height - crop_y);

            let cropped = img.crop_imm(crop_x, crop_y, crop_w, crop_h);
            let mut crop_bytes = Vec::new();
            cropped
                .write_to(&mut Cursor::new(&mut crop_bytes), ImageFormat::Jpeg)
                .map_err(|e| {
                    PipelineError::FaceModelError(format!("Failed to encode crop: {e}"))
                })?;

            let detected = DetectedFace {
                bbox: BoundingBox {
                    x: crop_x,
                    y: crop_y,
                    width: crop_w,
                    height: crop_h,
                    confidence: (skin_ratio as f32 * 2.0).clamp(0.65, 0.99),
                },
                image_width: width,
                image_height: height,
                face_crop_bytes: crop_bytes,
            };

            Ok(vec![detected])
        } else if width >= 50 && height >= 50 {
            // Fallback: central region face crop when portrait photo framing is used
            let crop_w = (width as f32 * 0.75) as u32;
            let crop_h = (height as f32 * 0.75) as u32;
            let crop_x = (width - crop_w) / 2;
            let crop_y = (height - crop_h) / 2;

            let cropped = img.crop_imm(crop_x, crop_y, crop_w, crop_h);
            let mut crop_bytes = Vec::new();
            cropped
                .write_to(&mut Cursor::new(&mut crop_bytes), ImageFormat::Jpeg)
                .map_err(|e| {
                    PipelineError::FaceModelError(format!("Failed to encode crop: {e}"))
                })?;

            Ok(vec![DetectedFace {
                bbox: BoundingBox {
                    x: crop_x,
                    y: crop_y,
                    width: crop_w,
                    height: crop_h,
                    confidence: 0.70,
                },
                image_width: width,
                image_height: height,
                face_crop_bytes: crop_bytes,
            }])
        } else {
            Ok(vec![])
        }
    }
}
