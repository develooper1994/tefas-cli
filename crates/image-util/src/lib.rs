use anyhow::Context;
use base64::Engine;
use std::fs;
use std::path::Path;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ImageOutputFormat {
    Png,
    Jpeg,
}

#[derive(Debug, Clone, Default)]
pub struct ImageConvertStats {
    pub wrote: usize,
    pub skipped: usize,
    pub errors: usize,
}

#[derive(Debug, Clone)]
pub struct ImageRecord {
    pub name: String,
    pub base64_payload: String,
}

pub fn sanitize_filename(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return "unnamed".to_string();
    }
    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "unnamed".to_string()
    } else {
        out
    }
}

pub fn decode_base64_payload(input: &str) -> anyhow::Result<Vec<u8>> {
    let mut encoded = input.trim().to_string();
    if encoded.starts_with("data:") && encoded.contains(',') {
        let (_, right) = encoded
            .split_once(',')
            .ok_or_else(|| anyhow::anyhow!("invalid data URI"))?;
        encoded = right.to_string();
    }

    match base64::engine::general_purpose::STANDARD.decode(encoded.as_bytes()) {
        Ok(bytes) => Ok(bytes),
        Err(_) => {
            let padding = (4 - (encoded.len() % 4)) % 4;
            if padding > 0 {
                encoded.push_str(&"=".repeat(padding));
            }
            base64::engine::general_purpose::STANDARD
                .decode(encoded.as_bytes())
                .with_context(|| "failed to decode base64 payload")
        }
    }
}

pub fn convert_base64_records_to_images(
    records: &[ImageRecord],
    outdir: &str,
    format: ImageOutputFormat,
    quality: u8,
    verbose: bool,
) -> anyhow::Result<ImageConvertStats> {
    let mut stats = ImageConvertStats::default();
    fs::create_dir_all(outdir)
        .with_context(|| format!("failed to create image output directory: {outdir}"))?;

    for (idx, rec) in records.iter().enumerate() {
        if rec.base64_payload.trim().is_empty() {
            stats.skipped += 1;
            continue;
        }

        let base = sanitize_filename(&rec.name);
        let raw_bytes = match decode_base64_payload(&rec.base64_payload) {
            Ok(bytes) => bytes,
            Err(err) => {
                eprintln!("[{idx}] Failed decoding base64 for '{}': {err}", rec.name);
                stats.errors += 1;
                continue;
            }
        };

        let img = match image::load_from_memory(&raw_bytes) {
            Ok(decoded) => decoded,
            Err(err) => {
                eprintln!("[{idx}] Failed loading image for '{}': {err}", rec.name);
                stats.errors += 1;
                continue;
            }
        };

        let (ext, out_path) = match format {
            ImageOutputFormat::Png => {
                let out = Path::new(outdir).join(format!("{base}.png"));
                ("png", out)
            }
            ImageOutputFormat::Jpeg => {
                let out = Path::new(outdir).join(format!("{base}.jpg"));
                ("jpg", out)
            }
        };

        let save_result = match format {
            ImageOutputFormat::Png => {
                let normalized = if matches!(
                    img.color(),
                    image::ColorType::Rgb8 | image::ColorType::Rgba8
                ) {
                    img
                } else {
                    image::DynamicImage::ImageRgba8(img.to_rgba8())
                };
                normalized.save_with_format(&out_path, image::ImageFormat::Png)
            }
            ImageOutputFormat::Jpeg => {
                let rgb = img.to_rgb8();
                let mut out_file = match fs::File::create(&out_path) {
                    Ok(f) => f,
                    Err(err) => {
                        eprintln!("[{idx}] Failed creating output for '{}': {err}", rec.name);
                        stats.errors += 1;
                        continue;
                    }
                };
                let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                    &mut out_file,
                    quality.max(1),
                );
                encoder.encode_image(&image::DynamicImage::ImageRgb8(rgb))
            }
        };

        if let Err(err) = save_result {
            eprintln!("[{idx}] Failed saving {ext} image for '{}': {err}", rec.name);
            stats.errors += 1;
            continue;
        }

        stats.wrote += 1;
        if verbose {
            println!("Wrote: {}", out_path.display());
        }
    }

    Ok(stats)
}
