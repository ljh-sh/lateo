//! Steganalysis self-check.
//!
//! Two classical LSB-steg detectors, both computed per colour channel (R, G, B):
//!
//! - **Chi-square statistic** (Westfeld–Pfitzmann 1999 style): group each
//!   channel's byte values into pairs `(2i, 2i+1)`. LSB embedding flattens
//!   the pair counts, so a **lower** χ² means the channel is more
//!   "equalised" and more suspicious.
//! - **LSB-equalised-pair fraction**: fraction of `(2i, 2i+1)` pairs whose
//!   counts differ by at most 1. In a natural image this is ~0.5; after LSB
//!   embedding with random data it approaches 1.0. Intuitive and easy to
//!   threshold.
//!
//! Plus a **bit-plane extractor** that renders any bit plane (0 = LSB …
//! 7 = MSB) of a channel as a black/white image so you can eyeball it.
//!
//! The verdict is a **heuristic** — these detectors are well-known to be
//! fooled by natural-image structure and by adaptive embedding. A "natural"
//! verdict is not a proof of innocence; a "likely stego" verdict is a reason
//! to look closer, not a conviction.

use image::RgbaImage;

/// Per-channel chi-square / equalisation result.
#[derive(Debug, Clone, Copy)]
pub struct ChannelStats {
    /// Sum of `(n_lo − n_hi)² / (n_lo + n_hi)` over the 128 value pairs
    /// `(2i, 2i+1)` in this channel. **Lower = more LSB-equalised.**
    pub chi_square: f64,
    /// Fraction of value pairs (with data) whose counts differ by at most 1.
    /// **Higher = more LSB-equalised.**
    pub equalised_fraction: f64,
}

/// Run chi-square + equalisation analysis on the R, G, B channels of `img`.
pub fn analyse(img: &RgbaImage) -> [ChannelStats; 3] {
    let mut hist = [[0u32; 256]; 3];
    for p in img.pixels() {
        for (c, slot) in hist.iter_mut().enumerate() {
            slot[p[c] as usize] += 1;
        }
    }
    let mut out = [ChannelStats {
        chi_square: 0.0,
        equalised_fraction: 0.0,
    }; 3];
    for (c, slot) in out.iter_mut().enumerate() {
        let mut chi = 0.0_f64;
        let mut pairs = 0u32;
        let mut equalised = 0u32;
        for pair in hist[c].chunks_exact(2) {
            let lo = pair[0];
            let hi = pair[1];
            let denom = (lo + hi) as f64;
            if denom > 0.0 {
                let diff = lo as f64 - hi as f64;
                chi += diff * diff / denom;
                pairs += 1;
                if (lo as i64 - hi as i64).abs() <= 1 {
                    equalised += 1;
                }
            }
        }
        *slot = ChannelStats {
            chi_square: chi,
            equalised_fraction: if pairs > 0 {
                equalised as f64 / pairs as f64
            } else {
                0.0
            },
        };
    }
    out
}

/// Heuristic overall verdict from the per-channel stats. Returns a short
/// stable string suitable for CLI output.
pub fn verdict(stats: &[ChannelStats; 3], threshold: f64) -> &'static str {
    let avg = stats.iter().map(|s| s.equalised_fraction).sum::<f64>() / 3.0;
    if avg >= threshold {
        "likely stego (LSB equalised)"
    } else {
        "probably natural"
    }
}

/// Render a single bit plane of the R channel as a black/white image
/// (bit 1 → white, bit 0 → black). `plane == 0` is the LSB.
pub fn bit_plane(img: &RgbaImage, plane: u8) -> RgbaImage {
    let (w, h) = img.dimensions();
    let mask = 1u8 << plane.min(7);
    let mut out = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y);
            let bit = (p[0] & mask) != 0;
            let v = if bit { 255 } else { 0 };
            out.put_pixel(x, y, image::Rgba([v, v, v, 0xFF]));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    fn blank(w: u32, h: u32, v: u8) -> RgbaImage {
        ImageBuffer::from_pixel(w, h, Rgba([v, v, v, 0xFF]))
    }

    #[test]
    fn uniform_image_is_not_equalised() {
        // A flat-gray image is "natural" by construction — only the value
        // pair containing `v` has data, and its counts are maximally
        // unequal (all on one side). So the equalised fraction is 0.
        let img = blank(128, 128, 0x80);
        let stats = analyse(&img);
        for (c, s) in stats.iter().enumerate() {
            assert!(
                s.equalised_fraction < 0.5,
                "uniform image should not look equalised on channel {c}, got {:?}",
                s
            );
        }
    }

    #[test]
    fn analyse_matches_known_histogram() {
        // Hand-computed reference: a 4×1 image whose per-channel
        // histograms and `(2i, 2i+1)` pair counts are easy to verify.
        //
        //   R: [0, 0, 2, 2]  → pair(0,1)=(2,0), pair(2,3)=(2,0)
        //   G: [0, 1, 0, 1]  → pair(0,1)=(2,2)
        //   B: [0, 0, 0, 0]  → pair(0,1)=(4,0)
        //
        // Expected per channel:
        //   R: χ² = (2²/2) + (2²/2)        = 4.0,  equalised = 0/2 = 0
        //   G: χ² = 0²/4                   = 0.0,  equalised = 1/1 = 1
        //   B: χ² = 4²/4                   = 4.0,  equalised = 0/1 = 0
        //
        // This pins the math (no randomness, no payload — a pure
        // function test of `analyse`).
        let mut img = RgbaImage::new(4, 1);
        img.put_pixel(0, 0, Rgba([0, 0, 0, 0xFF]));
        img.put_pixel(1, 0, Rgba([0, 1, 0, 0xFF]));
        img.put_pixel(2, 0, Rgba([2, 0, 0, 0xFF]));
        img.put_pixel(3, 0, Rgba([2, 1, 0, 0xFF]));
        let stats = analyse(&img);
        assert!(
            (stats[0].chi_square - 4.0).abs() < 1e-9,
            "R χ² = {} (expected 4.0)",
            stats[0].chi_square
        );
        assert_eq!(stats[0].equalised_fraction, 0.0);
        assert_eq!(stats[1].chi_square, 0.0);
        assert_eq!(stats[1].equalised_fraction, 1.0);
        assert!(
            (stats[2].chi_square - 4.0).abs() < 1e-9,
            "B χ² = {} (expected 4.0)",
            stats[2].chi_square
        );
        assert_eq!(stats[2].equalised_fraction, 0.0);
    }

    #[test]
    fn bit_plane_lsb_dimensions_and_values() {
        let mut img = RgbaImage::new(4, 1);
        // 0x00 → LSB 0 → black (0); 0x01 → LSB 1 → white (255).
        img.put_pixel(0, 0, Rgba([0x00, 0x00, 0x00, 0xFF]));
        img.put_pixel(1, 0, Rgba([0x01, 0x01, 0x01, 0xFF]));
        img.put_pixel(2, 0, Rgba([0x02, 0x02, 0x02, 0xFF]));
        img.put_pixel(3, 0, Rgba([0x03, 0x03, 0x03, 0xFF]));
        let plane = bit_plane(&img, 0);
        assert_eq!(plane.dimensions(), (4, 1));
        assert_eq!(plane.get_pixel(0, 0).0, [0, 0, 0, 0xFF]);
        assert_eq!(plane.get_pixel(1, 0).0, [255, 255, 255, 0xFF]);
        assert_eq!(plane.get_pixel(2, 0).0, [0, 0, 0, 0xFF]);
        assert_eq!(plane.get_pixel(3, 0).0, [255, 255, 255, 0xFF]);
    }
}
