//! Small dependency-free primitives shared by the engines:
//! - a seeded PRNG (SplitMix64) and a Fisher–Yates shuffle, for keyed
//!   position spreading;
//! - an FNV-1a hash to turn a passphrase key into a PRNG seed;
//! - CRC-32 (IEEE, reflected) for payload integrity.
//!
//! SplitMix64 is fast and deterministic but **not** a CSPRNG. It spreads bits
//! around so the envelope cannot be located without the key; it does not make
//! the payload secret. For content secrecy, encrypt-then-embed (a future
//! feature) is required.

/// FNV-1a 64-bit hash of a key into a PRNG seed.
pub fn hash_seed(key: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in key {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// SplitMix64 PRNG. Deterministic from a 64-bit seed.
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Next 64-bit output.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform-ish integer in `[0, n)`. Modulo bias is negligible for spreading.
    pub fn next_range(&mut self, n: u64) -> u64 {
        if n == 0 {
            return 0;
        }
        self.next_u64() % n
    }
}

/// A Fisher–Yates shuffle of `0..n`, seeded by `seed`. Produces a full
/// permutation, so every slot is visited exactly once — no collisions, no
/// reuse, regardless of payload size.
pub fn shuffled_indices(n: usize, seed: u64) -> Vec<u32> {
    let mut rng = SplitMix64::new(seed);
    let mut a: Vec<u32> = (0..n as u32).collect();
    for i in (1..n).rev() {
        let j = rng.next_range((i as u64) + 1) as usize;
        a.swap(i, j);
    }
    a
}

/// CRC-32 (IEEE 802.3, reflected), the same polynomial zlib/gzip use.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_known_vectors() {
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn shuffle_is_permutation() {
        let n = 1000;
        let perm = shuffled_indices(n, 42);
        let mut sorted = perm.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..n as u32).collect::<Vec<_>>());
    }

    #[test]
    fn shuffle_is_keyed() {
        // Same seed -> same permutation; different seed -> different.
        let a = shuffled_indices(500, 1);
        let b = shuffled_indices(500, 1);
        let c = shuffled_indices(500, 2);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn prng_is_deterministic() {
        let mut r1 = SplitMix64::new(7);
        let mut r2 = SplitMix64::new(7);
        for _ in 0..10 {
            assert_eq!(r1.next_u64(), r2.next_u64());
        }
    }
}
