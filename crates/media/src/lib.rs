//! TEFAS media adapter layer built on shared image utilities.

use image_util::{ImageRecord, convert_base64_records_to_images};
use serde_json::Value;

pub use image_util::{ImageConvertStats, ImageOutputFormat};

fn extract_base64_field_from_object(obj: &serde_json::Map<String, Value>) -> Option<String> {
    for key in [
        "base64Image",
        "base64LogoImage",
        "image",
        "logo",
        "img",
        "data",
    ] {
        if let Some(Value::String(s)) = obj.get(key)
            && !s.trim().is_empty()
        {
            return Some(s.to_string());
        }
    }

    for value in obj.values() {
        if let Value::Object(nested) = value {
            for key in ["base64Image", "img", "image", "logo", "data"] {
                if let Some(Value::String(s)) = nested.get(key)
                    && !s.trim().is_empty()
                {
                    return Some(s.to_string());
                }
            }
        }
    }

    None
}

/// Decode base64-encoded images from a `getLogo` API response and write them
/// to `outdir` as PNG or JPEG files.
pub fn convert_getlogo_to_images(
    raw: &Value,
    outdir: &str,
    format: ImageOutputFormat,
    quality: u8,
    verbose: bool,
) -> anyhow::Result<ImageConvertStats> {
    let records: Vec<Value> = match raw {
        Value::Array(a) => a.clone(),
        Value::Object(_) => vec![raw.clone()],
        _ => vec![],
    };

    let mut generic_records: Vec<ImageRecord> = Vec::new();
    let mut skipped = 0usize;

    for (idx, rec) in records.iter().enumerate() {
        let Some(obj) = rec.as_object() else {
            skipped += 1;
            continue;
        };

        let candidate_name = obj
            .get("memberCode")
            .and_then(Value::as_str)
            .or_else(|| obj.get("title").and_then(Value::as_str))
            .map_or_else(|| format!("logo_{idx}"), ToString::to_string);

        let Some(base64_image) = extract_base64_field_from_object(obj) else {
            if !verbose {
                eprintln!("[{idx}] No base64 image found for '{candidate_name}', skipping");
            }
            skipped += 1;
            continue;
        };

        generic_records.push(ImageRecord {
            name: candidate_name,
            base64_payload: base64_image,
        });
    }

    let mut stats =
        convert_base64_records_to_images(&generic_records, outdir, format, quality, verbose)?;
    stats.skipped += skipped;

    Ok(stats)
}
