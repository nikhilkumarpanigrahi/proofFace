use image::{ImageBuffer, ImageFormat, Rgb};
use std::fs;
use std::io::Cursor;
use std::path::Path;

fn main() {
    let samples_dir = Path::new("samples");
    if !samples_dir.exists() {
        fs::create_dir_all(samples_dir).unwrap();
    }

    let width = 300;
    let height = 300;

    // Create a portrait-style sample test image with facial region structure
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(width, height, |x, y| {
        let center_x = 150.0f32;
        let center_y = 150.0f32;

        let dx = (x as f32 - center_x) / 75.0;
        let dy = (y as f32 - center_y) / 100.0;
        let dist = dx * dx + dy * dy;

        if dist < 1.0 {
            // Face region: warm skin tone gradient
            let r = (220.0 - dist * 30.0) as u8;
            let g = (165.0 - dist * 25.0) as u8;
            let b = (130.0 - dist * 20.0) as u8;

            // Eyes region
            let is_left_eye = (x as f32 - 120.0).powi(2) + (y as f32 - 125.0).powi(2) < 80.0;
            let is_right_eye = (x as f32 - 180.0).powi(2) + (y as f32 - 125.0).powi(2) < 80.0;

            // Mouth region
            let is_mouth =
                (x as f32 - 150.0).powi(2) / 300.0 + (y as f32 - 185.0).powi(2) / 40.0 < 1.0;

            if is_left_eye || is_right_eye {
                Rgb([40, 30, 25])
            } else if is_mouth {
                Rgb([180, 80, 80])
            } else {
                Rgb([r, g, b])
            }
        } else {
            // Background gradient
            let bg = (240.0 - (y as f32 / height as f32) * 50.0) as u8;
            Rgb([bg, bg, bg + 10])
        }
    });

    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Jpeg)
        .unwrap();
    fs::write(samples_dir.join("input.jpg"), buf).unwrap();

    println!("✓ Generated samples/input.jpg");
}
