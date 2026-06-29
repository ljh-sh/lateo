//! Integration tests: full pipeline through real PNG files on disk.
//!
//! The unit tests exercise the engines on in-memory `RgbaImage` buffers; these
//! tests close the gap by routing through [`lateo::codec`] (PNG save → load),
//! which catches any lossless-round-trip or framing regression the in-memory
//! path would miss.

use image::{ImageBuffer, Rgba};
use lateo::{codec, steg, watermark, watermark_dct};
use std::path::PathBuf;

fn tmp(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("lateo_it_{}_{}", std::process::id(), name));
    p
}

fn cover() -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    ImageBuffer::from_pixel(128, 128, Rgba([0x80, 0x80, 0x80, 0xFF]))
}

#[test]
fn steg_round_trip_through_png_file() {
    let img = cover();
    let path = tmp("stego.png");
    let stego = steg::hide(&img, b"on-disk payload", b"k").unwrap();
    codec::save_png(&path, &stego).unwrap();
    let reloaded = codec::load(&path).unwrap();
    assert_eq!(
        steg::extract(&reloaded, b"k").unwrap(),
        Some(b"on-disk payload".to_vec())
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn steg_survives_png_resave_not_stego_file() {
    // A plain cover saved+reloaded must NOT yield a payload.
    let img = cover();
    let path = tmp("plain.png");
    codec::save_png(&path, &img).unwrap();
    let reloaded = codec::load(&path).unwrap();
    assert!(steg::extract(&reloaded, b"k").unwrap().is_none());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn fragile_watermark_through_png_file() {
    let img = cover();
    let path = tmp("fragile.png");
    let marked = watermark::mark(&img, b"owner:ljh-sh", b"k").unwrap();
    codec::save_png(&path, &marked).unwrap();
    let reloaded = codec::load(&path).unwrap();
    assert_eq!(
        watermark::verify(&reloaded, b"k").unwrap(),
        Some(b"owner:ljh-sh".to_vec())
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn robust_watermark_through_png_file() {
    // Robust watermark must survive a lossless PNG round-trip ~intactly.
    let img = cover();
    let path = tmp("robust.png");
    let marked = watermark_dct::mark(&img, b"owner:ljh-sh", b"k").unwrap();
    codec::save_png(&path, &marked).unwrap();
    let reloaded = codec::load(&path).unwrap();
    let rate = watermark_dct::verify(&reloaded, b"owner:ljh-sh", b"k").unwrap();
    assert!(
        rate > 0.99,
        "robust watermark should survive PNG round-trip, got {rate}"
    );
    let _ = std::fs::remove_file(&path);
}
