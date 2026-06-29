//! Steganography — a covert *message*, keyed-spread into LSBs.
//!
//! Fragile by design: it survives lossless round-trips (PNG re-save) but is
//! destroyed by any re-encoding or pixel edit. The payload is **not** encrypted
//! in v1: the key controls *where* bits go (Fisher–Yates spread), so the
//! envelope cannot be located without it, but a successful extraction yields
//! plaintext. Encrypt-then-embed is the planned path to real content secrecy
//! (see `SECURITY.md`).

use crate::lsb;
use image::RgbaImage;

/// Envelope magic for steganographic payloads (distinct from the watermark's).
const MAGIC: &[u8; 8] = b"LATEOSTG";

/// Embed a covert `payload` into `img`, keyed by `key`.
pub fn hide(img: &RgbaImage, payload: &[u8], key: &[u8]) -> crate::Result<RgbaImage> {
    lsb::embed(img, MAGIC, payload, key)
}

/// Recover a covert payload. `Ok(None)` = not embedded under this key, or
/// tampered.
pub fn extract(img: &RgbaImage, key: &[u8]) -> crate::Result<Option<Vec<u8>>> {
    lsb::extract(img, MAGIC, key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    fn blank(w: u32, h: u32) -> RgbaImage {
        ImageBuffer::from_pixel(w, h, Rgba([0x80, 0x80, 0x80, 0xFF]))
    }

    #[test]
    fn round_trip() {
        let img = blank(64, 64);
        let msg = b"hide me in the pixels";
        let stego = hide(&img, msg, b"secret-key").unwrap();
        let got = extract(&stego, b"secret-key")
            .unwrap()
            .expect("should extract");
        assert_eq!(got, msg);
    }

    #[test]
    fn wrong_key_yields_none() {
        let img = blank(64, 64);
        let stego = hide(&img, b"payload", b"key-a").unwrap();
        assert!(extract(&stego, b"key-b").unwrap().is_none());
    }

    #[test]
    fn plain_image_yields_none() {
        let img = blank(64, 64);
        assert!(extract(&img, b"any-key").unwrap().is_none());
    }

    #[test]
    fn oversize_errors() {
        let img = blank(2, 2); // 4 px × 3 = 12 bits capacity
        let big = vec![0u8; 100];
        assert!(hide(&img, &big, b"k").is_err());
    }

    #[test]
    fn alpha_is_preserved() {
        let mut img = blank(32, 32);
        // stamp a non-trivial alpha we expect to survive untouched.
        for p in img.pixels_mut() {
            p[3] = 0x7F;
        }
        let stego = hide(&img, b"x", b"k").unwrap();
        for p in stego.pixels() {
            assert_eq!(p[3], 0x7F);
        }
    }
}
