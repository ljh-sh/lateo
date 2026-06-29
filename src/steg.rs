//! Steganography — a covert *message*, keyed-spread into LSBs.
//!
//! Fragile by design: it survives lossless round-trips (PNG re-save) but is
//! destroyed by any re-encoding or pixel edit. The payload is **not** encrypted
//! in the default build: the key controls *where* bits go (Fisher–Yates
//! spread), so the envelope cannot be located without it, but a successful
//! extraction yields plaintext. With the `encryption` cargo feature, the
//! payload is sealed with chacha20poly1305 (keyed by an argon2id-derived key
//! from a passphrase) **before** embedding, providing content secrecy on top
//! of location secrecy — see [`hide_encrypted`].

use crate::lsb;
use image::RgbaImage;

/// Envelope magic for plaintext steganographic payloads.
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

/// Envelope magic for **encrypted** steganographic payloads. Distinct from
/// the plaintext magic so an encrypted envelope never reads as plaintext and
/// vice versa.
#[cfg(feature = "encryption")]
const MAGIC_ENC: &[u8; 8] = b"LATEOSGE";

/// Encrypt `plaintext` under `passphrase` (AEAD), then keyed-spread-embed
/// the resulting blob with `key`. The two are independent: `key` controls
/// *where* bits go, `passphrase` controls *what* the bits mean.
#[cfg(feature = "encryption")]
pub fn hide_encrypted(
    img: &RgbaImage,
    plaintext: &[u8],
    key: &[u8],
    passphrase: &[u8],
) -> crate::Result<RgbaImage> {
    let blob = crate::crypto::encrypt(plaintext, passphrase)?;
    crate::lsb::embed(img, MAGIC_ENC, &blob, key)
}

/// Decrypt the embedded payload. `None` = absent, wrong key, wrong passphrase
/// (AEAD tag mismatch), or not an encrypted envelope.
#[cfg(feature = "encryption")]
pub fn extract_encrypted(
    img: &RgbaImage,
    key: &[u8],
    passphrase: &[u8],
) -> crate::Result<Option<Vec<u8>>> {
    match crate::lsb::extract(img, MAGIC_ENC, key)? {
        Some(blob) => Ok(crate::crypto::decrypt(&blob, passphrase)?),
        None => Ok(None),
    }
}

/// Distinguish which envelope (if any) is present. `Some(false)` = plaintext,
/// `Some(true)` = encrypted, `None` = neither (wrong key / not a stego image).
/// Lets the CLI give a useful error when the user forgot `--passphrase`.
#[cfg(feature = "encryption")]
pub fn detect_kind(img: &RgbaImage, key: &[u8]) -> crate::Result<Option<bool>> {
    if crate::lsb::extract(img, MAGIC, key)?.is_some() {
        return Ok(Some(false));
    }
    if crate::lsb::extract(img, MAGIC_ENC, key)?.is_some() {
        return Ok(Some(true));
    }
    Ok(None)
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
        let img = blank(2, 2);
        let big = vec![0u8; 100];
        assert!(hide(&img, &big, b"k").is_err());
    }

    #[test]
    fn alpha_is_preserved() {
        let mut img = blank(32, 32);
        for p in img.pixels_mut() {
            p[3] = 0x7F;
        }
        let stego = hide(&img, b"x", b"k").unwrap();
        for p in stego.pixels() {
            assert_eq!(p[3], 0x7F);
        }
    }

    #[test]
    fn watermark_and_steg_do_not_collide() {
        use crate::watermark;
        let img = blank(64, 64);
        let stego = hide(&img, b"msg", b"k").unwrap();
        assert!(watermark::verify(&stego, b"k").unwrap().is_none());
        let marked = watermark::mark(&img, b"id", b"k").unwrap();
        assert!(extract(&marked, b"k").unwrap().is_none());
    }

    #[cfg(feature = "encryption")]
    #[test]
    fn encrypted_round_trip() {
        let img = blank(64, 64);
        let stego = hide_encrypted(&img, b"secret message", b"k", b"hunter2").unwrap();
        let got = extract_encrypted(&stego, b"k", b"hunter2")
            .unwrap()
            .expect("should decrypt");
        assert_eq!(got, b"secret message");
    }

    #[cfg(feature = "encryption")]
    #[test]
    fn encrypted_wrong_passphrase_yields_none() {
        let img = blank(64, 64);
        let stego = hide_encrypted(&img, b"secret", b"k", b"hunter2").unwrap();
        assert!(
            extract_encrypted(&stego, b"k", b"wrong-pass")
                .unwrap()
                .is_none(),
            "AEAD tag mismatch must yield None, not a wrong-but-readable plaintext"
        );
    }

    #[cfg(feature = "encryption")]
    #[test]
    fn encrypted_envelope_does_not_decrypt_without_passphrase() {
        let img = blank(64, 64);
        let stego = hide_encrypted(&img, b"secret", b"k", b"hunter2").unwrap();
        // Plaintext extract must NOT find an encrypted envelope.
        assert!(extract(&stego, b"k").unwrap().is_none());
        // detect_kind should report it as encrypted.
        assert!(detect_kind(&stego, b"k").unwrap().unwrap_or(false));
    }
}
