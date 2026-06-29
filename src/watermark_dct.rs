//! Robust watermark — a DCT-domain QIM imprint that survives mild lossy
//! re-encoding (e.g. moderate-quality JPEG).
//!
//! For each 8×8 luma block we quantise a mid-frequency DCT coefficient by
//! Quantisation Index Modulation (QIM, step Δ): the coefficient is forced to an
//! even multiple of Δ to encode `0` or an odd multiple to encode `1`. The
//! embedded bit-stream is a signature derived from `id` + `key`, one bit per
//! block. Verification re-reads each block's coefficient, recovers the bit, and
//! reports the fraction matching the expected signature — a robust watermark
//! survives JPEG at the cost of a few flipped blocks, so presence is decided
//! by a match-rate threshold rather than exact recovery.
//!
//! The 8×8 DCT basis is computed from `std::f64::cos` once per call (no
//! `rustdct` dependency). Δ is the imperceptibility ↔ robustness knob: larger
//! survives heavier compression but is more visible.

use crate::util::{hash_seed, SplitMix64};
use image::{Rgba, RgbaImage};

const BLOCK: usize = 8;
/// QIM quantisation step. ~24 survives JPEG quality ≥ ~50 on mid-frequency
/// coefficients while keeping visible artefacts modest.
const DELTA: f64 = 24.0;
/// Mid-frequency coefficient we modulate: robust to JPEG's quantisation table,
/// low visual impact.
const CU: usize = 3;
const CV: usize = 4;

type Mx = [[f64; BLOCK]; BLOCK];

fn luma(p: &Rgba<u8>) -> f64 {
    0.299 * p[0] as f64 + 0.587 * p[1] as f64 + 0.114 * p[2] as f64
}

fn clamp_byte(v: f64) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

/// Orthonormal DCT-II basis M (M·Mᵀ = I). Forward 2D DCT = M·F·Mᵀ;
/// inverse = Mᵀ·D·M.
fn dct_basis() -> Mx {
    let n = BLOCK as f64;
    let mut m = [[0.0f64; BLOCK]; BLOCK];
    for (u, row) in m.iter_mut().enumerate() {
        let alpha = if u == 0 {
            (1.0 / n).sqrt()
        } else {
            (2.0 / n).sqrt()
        };
        for (x, cell) in row.iter_mut().enumerate() {
            let theta = std::f64::consts::PI * (2 * x + 1) as f64 * u as f64 / (2.0 * n);
            *cell = alpha * theta.cos();
        }
    }
    m
}

fn transpose(m: &Mx) -> Mx {
    let mut t = [[0.0f64; BLOCK]; BLOCK];
    for i in 0..BLOCK {
        for j in 0..BLOCK {
            t[i][j] = m[j][i];
        }
    }
    t
}

fn mat_mul(a: &Mx, b: &Mx) -> Mx {
    let mut r = [[0.0f64; BLOCK]; BLOCK];
    for i in 0..BLOCK {
        for j in 0..BLOCK {
            let mut s = 0.0;
            for k in 0..BLOCK {
                s += a[i][k] * b[k][j];
            }
            r[i][j] = s;
        }
    }
    r
}

fn dct2(f: &Mx, m: &Mx) -> Mx {
    let mft = transpose(m);
    mat_mul(&mat_mul(m, f), &mft)
}

fn idct2(d: &Mx, m: &Mx) -> Mx {
    let mt = transpose(m);
    mat_mul(&mat_mul(&mt, d), m)
}

/// Deterministic signature bit-stream (one bit per block) bound to `id`+`key`.
fn signature_stream(id: &[u8], key: &[u8], nbits: usize) -> Vec<u8> {
    let mut seed = hash_seed(key);
    seed ^= hash_seed(id).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let mut rng = SplitMix64::new(seed);
    let nbytes = nbits.div_ceil(8);
    let mut out = Vec::with_capacity(nbytes);
    for _ in 0..nbytes {
        out.push(rng.next_u64() as u8);
    }
    out
}

fn sig_bit(stream: &[u8], i: usize) -> u8 {
    (stream[i / 8] >> (i % 8)) & 1
}

fn block_count(img: &RgbaImage) -> (usize, usize) {
    let (w, h) = img.dimensions();
    ((w as usize) / BLOCK, (h as usize) / BLOCK)
}

/// Embed a robust watermark bound to `id`+`key` into `img`.
pub fn mark(img: &RgbaImage, id: &[u8], key: &[u8]) -> crate::Result<RgbaImage> {
    if key.is_empty() {
        return Err("lateo: robust mark requires a non-empty key (--key)".into());
    }
    let (nbx, nby) = block_count(img);
    if nbx == 0 || nby == 0 {
        return Err("lateo: image too small for an 8x8-block DCT watermark".into());
    }
    let sig = signature_stream(id, key, nbx * nby);
    let m = dct_basis();
    let mut out = img.clone();
    for by in 0..nby {
        for bx in 0..nbx {
            qim_block(&mut out, bx, by, &m, sig_bit(&sig, by * nbx + bx));
        }
    }
    Ok(out)
}

/// Verify the robust watermark against `id`+`key`. Returns the fraction of
/// blocks whose recovered bit matches the expected signature (`0.0..=1.0`).
/// A pristine marked image returns ~1.0; an unmarked or wrong-key image ~0.5.
pub fn verify(img: &RgbaImage, id: &[u8], key: &[u8]) -> crate::Result<f64> {
    if key.is_empty() {
        return Err("lateo: robust verify requires a non-empty key (--key)".into());
    }
    let (nbx, nby) = block_count(img);
    let nbits = nbx * nby;
    if nbits == 0 {
        return Ok(0.0);
    }
    let sig = signature_stream(id, key, nbits);
    let m = dct_basis();
    let mut hits = 0usize;
    for by in 0..nby {
        for bx in 0..nbx {
            if read_bit(img, bx, by, &m) == sig_bit(&sig, by * nbx + bx) {
                hits += 1;
            }
        }
    }
    Ok(hits as f64 / nbits as f64)
}

fn qim_block(img: &mut RgbaImage, bx: usize, by: usize, m: &Mx, bit: u8) {
    let x0 = bx * BLOCK;
    let y0 = by * BLOCK;
    let mut f = [[0.0f64; BLOCK]; BLOCK];
    let mut orig = [[[0u8; 4]; BLOCK]; BLOCK];
    for (y, f_row) in f.iter_mut().enumerate() {
        let orig_row = &mut orig[y];
        for (x, f_cell) in f_row.iter_mut().enumerate() {
            let p = img.get_pixel((x0 + x) as u32, (y0 + y) as u32);
            orig_row[x] = [p[0], p[1], p[2], p[3]];
            *f_cell = luma(p);
        }
    }
    let d = dct2(&f, m);
    // QIM dither modulation: nearest even (bit 0) or odd (bit 1) multiple of Δ.
    let coeff = d[CU][CV];
    let q = coeff / DELTA;
    let idx = if bit == 0 {
        2.0 * (q / 2.0).round()
    } else {
        2.0 * ((q - 1.0) / 2.0).round() + 1.0
    };
    let mut d2 = d;
    d2[CU][CV] = idx * DELTA;
    let f2 = idct2(&d2, m);
    // Write luma delta back into R/G/B equally (shifts Y by exactly delta),
    // preserving chroma; clamp to valid byte range.
    for (y, f2_row) in f2.iter().enumerate() {
        for (x, &fv2) in f2_row.iter().enumerate() {
            let dy = fv2 - f[y][x];
            let [r, g, b, a] = orig[y][x];
            img.put_pixel(
                (x0 + x) as u32,
                (y0 + y) as u32,
                Rgba([
                    clamp_byte(r as f64 + dy),
                    clamp_byte(g as f64 + dy),
                    clamp_byte(b as f64 + dy),
                    a,
                ]),
            );
        }
    }
}

fn read_bit(img: &RgbaImage, bx: usize, by: usize, m: &Mx) -> u8 {
    let x0 = bx * BLOCK;
    let y0 = by * BLOCK;
    let mut f = [[0.0f64; BLOCK]; BLOCK];
    for (y, row) in f.iter_mut().enumerate() {
        for (x, cell) in row.iter_mut().enumerate() {
            *cell = luma(img.get_pixel((x0 + x) as u32, (y0 + y) as u32));
        }
    }
    let d = dct2(&f, m);
    let q = (d[CU][CV] / DELTA).round();
    q.rem_euclid(2.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    fn blank(w: u32, h: u32) -> RgbaImage {
        ImageBuffer::from_pixel(w, h, Rgba([0x80, 0x80, 0x80, 0xFF]))
    }

    #[test]
    fn mark_then_verify_full_match() {
        let img = blank(64, 64);
        let marked = mark(&img, b"ljh-sh", b"k").unwrap();
        let m = verify(&marked, b"ljh-sh", b"k").unwrap();
        assert!(
            m > 0.99,
            "pristine marked image should match ~100%, got {m}"
        );
    }

    #[test]
    fn unmarked_about_half() {
        let img = blank(64, 64);
        let m = verify(&img, b"ljh-sh", b"k").unwrap();
        assert!(m < 0.65, "unmarked image should be ~50%, got {m}");
    }

    #[test]
    fn wrong_id_low_match() {
        let img = blank(64, 64);
        let marked = mark(&img, b"ljh-sh", b"k").unwrap();
        let m = verify(&marked, b"someone-else", b"k").unwrap();
        assert!(m < 0.65, "wrong id should be ~50%, got {m}");
    }

    #[test]
    fn survives_small_luma_perturbation() {
        // QIM step Δ=24 survives per-pixel luma shifts up to ~Δ/2.
        let img = blank(64, 64);
        let mut marked = mark(&img, b"ljh-sh", b"k").unwrap();
        for p in marked.pixels_mut() {
            p[0] = p[0].saturating_add(3);
            p[1] = p[1].saturating_add(3);
            p[2] = p[2].saturating_add(3);
        }
        let m = verify(&marked, b"ljh-sh", b"k").unwrap();
        assert!(
            m > 0.9,
            "small perturbation should not flip many blocks, got {m}"
        );
    }
}
