use serde_json::json;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tefas_media::{convert_getlogo_to_images, ImageOutputFormat};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{}_{}_{}", prefix, std::process::id(), nanos));
    fs::create_dir_all(&dir).expect("failed to create temp dir");
    dir
}

fn png_base64_1x1() -> String {
    use base64::Engine;
    let rgba = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]));
    let dyn_img = image::DynamicImage::ImageRgba8(rgba);
    let mut buf = Vec::new();
    dyn_img
        .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
        .expect("failed to write in-memory png");
    base64::engine::general_purpose::STANDARD.encode(buf)
}

#[test]
fn writes_png_image_from_array_payload() {
    let outdir = unique_temp_dir("tefas_media_png");
    let png_b64 = png_base64_1x1();
    let payload = json!([
        {
            "memberCode": "AC5",
            "base64Image": png_b64
        }
    ]);

    let stats = convert_getlogo_to_images(
        &payload,
        outdir.to_str().expect("utf8 path"),
        ImageOutputFormat::Png,
        85,
        false,
    )
    .expect("conversion must succeed");

    assert_eq!(stats.wrote, 1);
    assert_eq!(stats.skipped, 0);
    assert_eq!(stats.errors, 0);
    assert!(outdir.join("AC5.png").exists());

    let _ = fs::remove_dir_all(outdir);
}

#[test]
fn writes_jpeg_image_from_data_uri() {
    let outdir = unique_temp_dir("tefas_media_jpg");
    let png_b64 = png_base64_1x1();
    let payload = json!({
        "title": "TLY",
        "image": format!("data:image/png;base64,{}", png_b64)
    });

    let stats = convert_getlogo_to_images(
        &payload,
        outdir.to_str().expect("utf8 path"),
        ImageOutputFormat::Jpeg,
        90,
        false,
    )
    .expect("conversion must succeed");

    assert_eq!(stats.wrote, 1);
    assert_eq!(stats.skipped, 0);
    assert_eq!(stats.errors, 0);
    assert!(outdir.join("TLY.jpg").exists());

    let _ = fs::remove_dir_all(outdir);
}

#[test]
fn reports_error_on_invalid_base64() {
    let outdir = unique_temp_dir("tefas_media_bad_b64");
    let payload = json!([
        {
            "memberCode": "BAD",
            "base64Image": "%%%NOT_BASE64%%%"
        }
    ]);

    let stats = convert_getlogo_to_images(
        &payload,
        outdir.to_str().expect("utf8 path"),
        ImageOutputFormat::Png,
        85,
        false,
    )
    .expect("function should not fail hard on bad record");

    assert_eq!(stats.wrote, 0);
    assert_eq!(stats.errors, 1);
    assert!(!outdir.join("BAD.png").exists());

    let _ = fs::remove_dir_all(outdir);
}

#[test]
fn skips_records_without_image_fields() {
    let outdir = unique_temp_dir("tefas_media_skip");
    let payload = json!([
        {
            "memberCode": "NOIMG"
        }
    ]);

    let stats = convert_getlogo_to_images(
        &payload,
        outdir.to_str().expect("utf8 path"),
        ImageOutputFormat::Png,
        85,
        false,
    )
    .expect("conversion should succeed with skipped records");

    assert_eq!(stats.wrote, 0);
    assert_eq!(stats.skipped, 1);
    assert_eq!(stats.errors, 0);

    let _ = fs::remove_dir_all(outdir);
}
