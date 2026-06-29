//! Keyed-spread LSB embedding — the shared core of steganography and the
//! fragile watermark. Both write a framed payload into the LSBs of the R/G/B
//! channels at positions determined by a Fisher–Yates shuffle seeded from the
//! key; they differ only in the 8-byte magic that marks their envelope, so a
//! steg extract never accidentally reads a watermark and vice versa.

use crate::util::{crc32, hash_seed, shuffled_indices};
use image::RgbaImage;

/// Envelope: `[magic 8B][payload_len u32 BE][payload][crc32 4B]`.
const HEADER_LEN: usize = 8 + 4;
const TRAILER_LEN: usize = 4;

/// Number of payload-carrying LSB slots (R, G, B per pixel; alpha untouched).
pub fn capacity_bits(img: &RgbaImage) -> usize {
    let (w, h) = img.dimensions();
    w as usize * h as usize * 3
}

fn envelope_len(payload_len: usize) -> usize {
    HEADER_LEN + payload_len + TRAILER_LEN
}

/// Embed `payload` under `magic` into `img`, keyed by `key`. Returns a new
/// image. Errors if the payload does not fit or the key is empty.
pub fn embed(
    img: &RgbaImage,
    magic: &[u8; 8],
    payload: &[u8],
    key: &[u8],
) -> crate::Result<RgbaImage> {
    if key.is_empty() {
        return Err("lateo: embedding requires a non-empty key (--key)".into());
    }
    let env_len = envelope_len(payload.len());
    let need = env_len * 8;
    let cap = capacity_bits(img);
    if need > cap {
        return Err(
            format!("lateo: payload too large — need {need} bits, image holds {cap}").into(),
        );
    }

    let mut env = Vec::with_capacity(env_len);
    env.extend_from_slice(magic);
    env.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    env.extend_from_slice(payload);
    env.extend_from_slice(&crc32(payload).to_be_bytes());

    let order = shuffled_indices(cap, hash_seed(key));
    let (w, _h) = img.dimensions();
    let mut out = img.clone();
    for (bit_i, byte) in env.iter().enumerate() {
        for b in 0..8 {
            let bit = (byte >> (7 - b)) & 1;
            set_lsb(&mut out, w, order[bit_i * 8 + b] as usize, bit);
        }
    }
    Ok(out)
}

/// Recover the payload for `magic` under `key`. Returns `Ok(None)` if the
/// magic is absent (wrong key / not embedded / tampered), so callers treat it
/// as "not found" rather than a hard error.
pub fn extract(img: &RgbaImage, magic: &[u8; 8], key: &[u8]) -> crate::Result<Option<Vec<u8>>> {
    if key.is_empty() {
        return Err("lateo: extraction requires a non-empty key (--key)".into());
    }
    let cap = capacity_bits(img);
    if cap < HEADER_LEN * 8 {
        return Ok(None); // image too small to hold even a header
    }
    let (w, _h) = img.dimensions();
    let order = shuffled_indices(cap, hash_seed(key));

    // Phase 1: header (magic + length).
    let mut header = [0u8; HEADER_LEN];
    for bit_i in 0..(HEADER_LEN * 8) {
        let bit = get_lsb(img, w, order[bit_i] as usize);
        header[bit_i / 8] |= bit << (7 - (bit_i % 8));
    }
    if &header[..8] != magic {
        return Ok(None);
    }
    let payload_len = u32::from_be_bytes([header[8], header[9], header[10], header[11]]) as usize;
    let env_len = envelope_len(payload_len);
    if env_len > cap / 8 + 1 {
        return Ok(None); // implausible length → wrong-key garbage
    }

    // Phase 2: payload + crc.
    let rest_len = env_len - HEADER_LEN;
    let mut rest = vec![0u8; rest_len];
    let base = HEADER_LEN * 8;
    for bit_i in 0..(rest_len * 8) {
        let bit = get_lsb(img, w, order[base + bit_i] as usize);
        rest[bit_i / 8] |= bit << (7 - (bit_i % 8));
    }
    let payload = &rest[..payload_len];
    let c = payload_len;
    let stored = u32::from_be_bytes([rest[c], rest[c + 1], rest[c + 2], rest[c + 3]]);
    if crc32(payload) != stored {
        return Ok(None); // tampered / wrong key
    }
    Ok(Some(payload.to_vec()))
}

fn set_lsb(img: &mut RgbaImage, w: u32, slot: usize, bit: u8) {
    let p = slot / 3;
    let ch = slot % 3;
    let x = (p as u32) % w;
    let y = (p as u32) / w;
    let px = img.get_pixel_mut(x, y);
    px[ch] = (px[ch] & 0xFE) | (bit & 1);
}

fn get_lsb(img: &RgbaImage, w: u32, slot: usize) -> u8 {
    let p = slot / 3;
    let ch = slot % 3;
    let x = (p as u32) % w;
    let y = (p as u32) / w;
    img.get_pixel(x, y)[ch] & 1
}
