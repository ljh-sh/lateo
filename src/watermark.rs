//! Watermark — a robust *imprint* asserting ownership / provenance.
//!
//! This is the **fragile / semi-fragile** variant: it reuses the keyed-spread
//! LSB core ([`crate::lsb`]) carrying a short owner `id` rather than an
//! arbitrary message. It survives lossless round-trips (PNG re-save) but breaks
//! on any pixel edit or lossy re-encoding — which is exactly the property that
//! makes it a **tamper detector**: [`verify`] returns the id only if the image
//! is pristine. For a watermark that survives mild lossy compression (JPEG),
//! see [`crate::watermark_dct`].

use crate::lsb;
use image::RgbaImage;

/// Envelope magic for fragile watermarks (distinct from steganography's).
const MAGIC: &[u8; 8] = b"LATEOWMK";

/// Embed an owner `id` as a fragile watermark, keyed by `key`.
pub fn mark(img: &RgbaImage, id: &[u8], key: &[u8]) -> crate::Result<RgbaImage> {
    lsb::embed(img, MAGIC, id, key)
}

/// Verify the fragile watermark. `Ok(Some(id))` = imprint intact;
/// `Ok(None)` = absent, wrong key, or tampered (CRC mismatch).
pub fn verify(img: &RgbaImage, key: &[u8]) -> crate::Result<Option<Vec<u8>>> {
    lsb::extract(img, MAGIC, key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    fn blank(w: u32, h: u32) -> RgbaImage {
        ImageBuffer::from_pixel(w, h, Rgba([0x12, 0x34, 0x56, 0xFF]))
    }

    #[test]
    fn mark_then_verify() {
        let img = blank(64, 64);
        let marked = mark(&img, b"owner:ljh-sh", b"k").unwrap();
        assert_eq!(
            verify(&marked, b"k").unwrap(),
            Some(b"owner:ljh-sh".to_vec())
        );
    }

    #[test]
    fn tamper_breaks_verify() {
        let img = blank(64, 64);
        let mut marked = mark(&img, b"owner:ljh-sh", b"k").unwrap();
        // flip every LSB — a fragile watermark must break.
        for p in marked.pixels_mut() {
            p[0] ^= 1;
            p[1] ^= 1;
            p[2] ^= 1;
        }
        assert_eq!(verify(&marked, b"k").unwrap(), None);
    }

    #[test]
    fn unmarked_image_yields_none() {
        let img = blank(64, 64);
        assert!(verify(&img, b"k").unwrap().is_none());
    }

    #[test]
    fn watermark_and_steg_do_not_collide() {
        // A stego image must not verify as a watermark and vice versa.
        use crate::steg;
        let img = blank(64, 64);
        let stego = steg::hide(&img, b"msg", b"k").unwrap();
        assert!(verify(&stego, b"k").unwrap().is_none());
        let marked = mark(&img, b"id", b"k").unwrap();
        assert!(steg::extract(&marked, b"k").unwrap().is_none());
    }
}
