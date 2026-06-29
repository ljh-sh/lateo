//! Optional encrypt-then-embed: chacha20poly1305 AEAD + argon2id KDF.
//!
//! Only compiled with the `encryption` cargo feature (off by default, so the
//! no-crypto build stays zero-cost — invariant #4). The encrypted payload
//! is a self-describing blob:
//!
//! ```text
//! | version(1) | salt(16) | nonce(12) | ciphertext+poly1305-tag(plaintext.len+16) |
//! ```
//!
//! The first byte is the format version; `decrypt` rejects anything else, so
//! a wrong passphrase yields a decryption failure (AEAD auth-tag mismatch)
//! rather than a wrong-but-readable plaintext — the right default for
//! content secrecy.

#[cfg(feature = "encryption")]
mod inner {
    use argon2::{Algorithm, Argon2, Params};
    use chacha20poly1305::aead::{Aead, KeyInit};
    use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};

    const VERSION: u8 = 0x01;
    const SALT_LEN: usize = 16;
    const NONCE_LEN: usize = 12;
    const KEY_LEN: usize = 32;
    /// version(1) + salt(16) + nonce(12).
    pub const HEADER_LEN: usize = 1 + SALT_LEN + NONCE_LEN;

    fn kdf(
        passphrase: &[u8],
        salt: &[u8],
    ) -> Result<Key, Box<dyn std::error::Error + Send + Sync>> {
        // 7 MiB / t=3 / p=1 → ~50 ms — comfortable for a CLI, well above the
        // weak-passphrase floor argon2id is meant to provide.
        let params = Params::new(7 * 1024, 3, 1, Some(KEY_LEN))
            .map_err(|e| format!("argon2 params: {e}"))?;
        let argon = Argon2::new(Algorithm::Argon2id, argon2::Version::V0x13, params);
        let mut out = [0u8; KEY_LEN];
        argon
            .hash_password_into(passphrase, salt, &mut out)
            .map_err(|e| format!("argon2 kdf: {e}"))?;
        Ok(*Key::from_slice(&out))
    }

    /// Encrypt `plaintext` under `passphrase`. Returns a self-describing blob.
    pub fn encrypt(
        plaintext: &[u8],
        passphrase: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let mut salt = [0u8; SALT_LEN];
        let mut nonce = [0u8; NONCE_LEN];
        getrandom::fill(&mut salt).map_err(|e| format!("getrandom salt: {e}"))?;
        getrandom::fill(&mut nonce).map_err(|e| format!("getrandom nonce: {e}"))?;
        let key = kdf(passphrase, &salt)?;
        let cipher = ChaCha20Poly1305::new(&key);
        let ct = cipher
            .encrypt(Nonce::from_slice(&nonce), plaintext)
            .map_err(|e| format!("encrypt: {e}"))?;
        let mut out = Vec::with_capacity(HEADER_LEN + ct.len());
        out.push(VERSION);
        out.extend_from_slice(&salt);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ct);
        Ok(out)
    }

    /// Decrypt a self-describing blob produced by [`encrypt`]. Returns
    /// `None` for any failure (wrong version, truncated, wrong passphrase →
    /// AEAD tag mismatch) so callers surface "not found" rather than a
    /// distinct error for each failure mode.
    pub fn decrypt(
        blob: &[u8],
        passphrase: &[u8],
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        if blob.len() < HEADER_LEN || blob[0] != VERSION {
            return Ok(None);
        }
        let salt = &blob[1..1 + SALT_LEN];
        let nonce = &blob[1 + SALT_LEN..HEADER_LEN];
        let ct = &blob[HEADER_LEN..];
        let key = kdf(passphrase, salt)?;
        let cipher = ChaCha20Poly1305::new(&key);
        match cipher.decrypt(Nonce::from_slice(nonce), ct) {
            Ok(pt) => Ok(Some(pt)),
            Err(_) => Ok(None), // wrong passphrase / corrupted
        }
    }
}

#[cfg(feature = "encryption")]
pub use inner::{decrypt, encrypt, HEADER_LEN as ENCRYPTED_HEADER_LEN};
