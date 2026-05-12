use image::codecs::png::{CompressionType, FilterType as PngFilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder, RgbaImage};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    pub const WHITE: Self = Self::rgba(255, 255, 255, 255);
    pub const BLACK: Self = Self::rgba(0, 0, 0, 255);

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PngCompressionMode {
    #[default]
    Default,
    Fast,
    Fastest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PngWriteOptions {
    #[serde(default)]
    pub compression: PngCompressionMode,
}

impl Default for PngWriteOptions {
    fn default() -> Self {
        Self {
            compression: PngCompressionMode::Default,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PngWriteTiming {
    pub png_encode_ms: u128,
    pub png_write_ms: u128,
    pub total_ms: u128,
}

pub fn save_rgba_png_profile_with_options(
    image: &RgbaImage,
    output_path: impl AsRef<Path>,
    png_options: &PngWriteOptions,
) -> Result<PngWriteTiming, Box<dyn std::error::Error>> {
    let total_start = Instant::now();
    let encode_start = Instant::now();
    let (compression, filter) = match png_options.compression {
        PngCompressionMode::Default => (CompressionType::Default, PngFilterType::Up),
        PngCompressionMode::Fast => (CompressionType::Fast, PngFilterType::Up),
        PngCompressionMode::Fastest => (CompressionType::Fast, PngFilterType::NoFilter),
    };
    let mut bytes = Vec::new();
    let encoder = PngEncoder::new_with_quality(&mut bytes, compression, filter);
    encoder.write_image(
        image.as_raw(),
        image.width(),
        image.height(),
        ExtendedColorType::Rgba8,
    )?;
    let png_encode_ms = encode_start.elapsed().as_millis();

    let path = output_path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let write_start = Instant::now();
    std::fs::write(path, bytes)?;
    Ok(PngWriteTiming {
        png_encode_ms,
        png_write_ms: write_start.elapsed().as_millis(),
        total_ms: total_start.elapsed().as_millis(),
    })
}
