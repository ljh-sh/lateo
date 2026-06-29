//! lateo — imperceptible data embedding for images.
//!
//! Two complementary primitives share one engine:
//!
//! - **Steganography** ([`steg`], `hide` / `extract`): a high-capacity covert
//!   *message*. Existence is denied; the payload is fragile (minimises
//!   statistical detectability, does not survive re-encoding).
//! - **Watermarking** ([`watermark`], `mark` / `verify`): a low-capacity robust
//!   *imprint*. Existence is asserted; the payload survives re-encoding and
//!   identifies ownership / provenance.
//!
//! The distinction is what is hidden: steganography hides a *message*,
//! watermarking hides an *imprint*. The name is Latin *lateo* — "I lie hidden"
//! — the root of *latent*.

#![forbid(unsafe_code)]

pub mod codec;
#[cfg(feature = "encryption")]
pub mod crypto;
pub mod lsb;
pub mod steg;
pub mod util;
pub mod watermark;
pub mod watermark_dct;

/// Boxed error. Keeps v1 dependency-light (no `thiserror`): engines return
/// [`Result<T>`] over this, and the CLI reports the message verbatim.
pub type Error = Box<dyn std::error::Error + Send + Sync>;
/// Shorthand result used across the engines.
pub type Result<T> = std::result::Result<T, Error>;

/// Crate version, surfaced by `lateo --version`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// An operation `lateo` can perform. Each variant maps 1:1 to a CLI verb.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Embed a covert payload. Steganography; fragile, high-capacity.
    Hide,
    /// Recover a covert payload.
    Extract,
    /// Embed a robust ownership / provenance imprint. Watermarking.
    Mark,
    /// Verify a robust imprint is present / has survived.
    Verify,
    /// Reverse self-check: can the embedding be detected or stripped?
    Probe,
}

impl Op {
    /// Parse a CLI verb. Returns `None` for an unknown verb.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "hide" => Some(Self::Hide),
            "extract" => Some(Self::Extract),
            "mark" => Some(Self::Mark),
            "verify" => Some(Self::Verify),
            "probe" => Some(Self::Probe),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Op;

    #[test]
    fn parse_known_and_unknown_verbs() {
        assert_eq!(Op::parse("hide"), Some(Op::Hide));
        assert_eq!(Op::parse("extract"), Some(Op::Extract));
        assert_eq!(Op::parse("mark"), Some(Op::Mark));
        assert_eq!(Op::parse("verify"), Some(Op::Verify));
        assert_eq!(Op::parse("probe"), Some(Op::Probe));
        assert_eq!(Op::parse("watermark"), None);
        assert_eq!(Op::parse(""), None);
    }
}
