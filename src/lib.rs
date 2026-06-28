//! lateo — imperceptible data embedding for images.
//!
//! Two complementary primitives share one engine:
//!
//! - **Steganography** (`hide` / `extract`): a high-capacity covert *message*.
//!   Existence is denied; the payload is fragile (minimises statistical
//!   detectability, does not survive re-encoding).
//! - **Watermarking** (`mark` / `verify`): a low-capacity robust *imprint*.
//!   Existence is asserted; the payload survives re-encoding and identifies
//!   ownership / provenance.
//!
//! The distinction is what is hidden: steganography hides a *message*,
//! watermarking hides an *imprint*. The name is Latin *lateo* — "I lie
//! hidden" — the root of *latent*.
//!
//! Status: scaffold. The CLI verbs are wired ([`Op`]) but the embedding
//! engines are not yet implemented.

#![forbid(unsafe_code)]

/// Crate version, surfaced by `lateo --version`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// An operation `lateo` can perform. Each variant maps 1:1 to a CLI verb.
///
/// Steganography and watermarking are kept as distinct operations rather than
/// a single "hide data" call: their optimisation targets are opposed
/// (imperceptibility-and-capacity vs. robustness), so they need separate
/// engines even though they share the image I/O and transform plumbing.
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
    /// Parse a CLI verb. Returns `None` for an unknown verb so the caller can
    /// emit a uniform "unknown subcommand" diagnostic.
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
