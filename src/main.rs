//! lateo CLI — imperceptible data embedding (steganography + watermarking).
//!
//! Subcommands dispatch to the engines in `lateo`:
//! - `hide` / `extract` → [`lateo::steg`] (LSB keyed-spread, fragile)
//! - `mark` / `verify --mode fragile` → [`lateo::watermark`] (tamper detection)
//! - `mark` / `verify --mode robust`  → [`lateo::watermark_dct`] (survives mild JPEG)
//! - `scout`  → [`lateo::probe::scout`]  (carrier quality)
//! - `probe`  → [`lateo::probe::analyse`] (steganalysis self-check)
//!
//! `--passphrase` on `hide` / `extract` activates encrypt-then-embed (the
//! `encryption` cargo feature must be enabled at build time).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "lateo",
    version = lateo::VERSION,
    about = "Imperceptible data embedding — steganography (covert payload) + watermarking (robust imprint)"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Embed a covert payload (steganography — fragile, high-capacity).
    Hide {
        /// Carrier image (PNG; lossless round-trip required).
        #[arg(short, long)]
        image: PathBuf,
        /// Stego key. Determines where bits go; required to extract.
        #[arg(short, long)]
        key: String,
        /// Payload as a literal string.
        #[arg(short, long)]
        message: Option<String>,
        /// Payload read from a file (binary-safe).
        #[arg(short = 'M', long = "message-file")]
        message_file: Option<PathBuf>,
        /// Encrypt the payload with this passphrase (chacha20poly1305) before
        /// embedding. Requires building with `--features encryption`.
        #[arg(long)]
        passphrase: Option<String>,
        /// Output PNG (default: <image>.lateo.png).
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Recover a covert payload.
    Extract {
        #[arg(short, long)]
        image: PathBuf,
        #[arg(short, long)]
        key: String,
        /// Decrypt with this passphrase (requires `--features encryption`).
        #[arg(long)]
        passphrase: Option<String>,
    },
    /// Embed an ownership imprint (watermark).
    Mark {
        #[arg(short, long)]
        image: PathBuf,
        /// Owner / provenance identifier to embed.
        #[arg(long)]
        id: String,
        #[arg(short, long)]
        key: String,
        /// fragile = tamper detection (survives lossless, breaks on edit);
        /// robust = DCT imprint, survives mild lossy re-encoding (JPEG).
        #[arg(long, value_enum, default_value_t = Mode::Fragile)]
        mode: Mode,
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Verify an imprint.
    Verify {
        #[arg(short, long)]
        image: PathBuf,
        #[arg(short, long)]
        key: String,
        #[arg(long, value_enum, default_value_t = Mode::Fragile)]
        mode: Mode,
        /// Required for robust mode (signature is bound to id+key).
        #[arg(long)]
        id: Option<String>,
    },
    /// Score an image's suitability as a steg carrier (capacity + baseline
    /// stats + heuristic verdict).
    Scout {
        #[arg(short, long)]
        image: PathBuf,
    },
    /// Steganalysis self-check (chi-square + LSB-equalised + bit-plane).
    /// Inverse of `scout`: detects whether something is already embedded.
    Probe {
        /// Image to analyse.
        #[arg(short, long)]
        image: PathBuf,
        /// Bit plane to extract (0 = LSB … 7 = MSB). Only used with --out.
        #[arg(long, default_value_t = 0u8)]
        plane: u8,
        /// Write the chosen bit plane as a PNG. Default:
        /// `<image>.probe.plane<N>.png`.
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Mode {
    /// Spatial LSB signature — presence + tamper detection.
    Fragile,
    /// DCT-domain QIM — survives mild lossy compression.
    Robust,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("lateo: {e}");
            ExitCode::FAILURE
        }
    }
}

const PRESENT_THRESHOLD: f64 = 0.75;

#[allow(clippy::needless_return)] // the cfg-gated `return Err("--passphrase ...")` blocks are NOT the function tail
fn run(cli: Cli) -> lateo::Result<()> {
    match cli.cmd {
        Cmd::Hide {
            image,
            key,
            message,
            message_file,
            passphrase,
            out,
        } => {
            let payload = read_payload(message, message_file)?;
            let img = lateo::codec::load(&image)?;
            let (out_path, label) = if let Some(pass) = passphrase.as_deref() {
                #[cfg(not(feature = "encryption"))]
                {
                    let _ = pass;
                    return Err(
                        "--passphrase requires building with `--features encryption`".into(),
                    );
                }
                #[cfg(feature = "encryption")]
                {
                    let stego = lateo::steg::hide_encrypted(
                        &img,
                        &payload,
                        key.as_bytes(),
                        pass.as_bytes(),
                    )?;
                    let out = out.unwrap_or_else(|| default_out(&image));
                    lateo::codec::save_png(&out, &stego)?;
                    (out, "encrypted-hid")
                }
            } else {
                let stego = lateo::steg::hide(&img, &payload, key.as_bytes())?;
                let out = out.unwrap_or_else(|| default_out(&image));
                lateo::codec::save_png(&out, &stego)?;
                (out, "hid")
            };
            eprintln!(
                "lateo: {label} {} bytes (capacity {} bits) -> {}",
                payload.len(),
                lateo::lsb::capacity_bits(&img),
                out_path.display()
            );
            Ok(())
        }
        Cmd::Extract {
            image,
            key,
            passphrase,
        } => {
            let img = lateo::codec::load(&image)?;
            if let Some(pass) = passphrase.as_deref() {
                #[cfg(not(feature = "encryption"))]
                {
                    let _ = pass;
                    return Err(
                        "--passphrase requires building with `--features encryption`".into(),
                    );
                }
                #[cfg(feature = "encryption")]
                {
                    match lateo::steg::extract_encrypted(&img, key.as_bytes(), pass.as_bytes())? {
                        Some(payload) => {
                            let mut out = std::io::stdout().lock();
                            out.write_all(&payload)?;
                            out.flush()?;
                            Ok(())
                        }
                        None => Err(
                            "no encrypted payload found (wrong key, wrong passphrase, or tampered)"
                                .into(),
                        ),
                    }
                }
            } else {
                if let Some(payload) = lateo::steg::extract(&img, key.as_bytes())? {
                    let mut out = std::io::stdout().lock();
                    out.write_all(&payload)?;
                    out.flush()?;
                } else {
                    // Not plaintext — give a useful error if it's an encrypted
                    // envelope, otherwise the generic "not found".
                    #[cfg(feature = "encryption")]
                    {
                        if lateo::steg::detect_kind(&img, key.as_bytes())?.unwrap_or(false) {
                            return Err(
                                "this envelope is encrypted; pass --passphrase to decrypt".into()
                            );
                        }
                    }
                    return Err(
                        "no payload found (wrong key, not a stego image, or tampered)".into(),
                    );
                }
                Ok(())
            }
        }
        Cmd::Mark {
            image,
            id,
            key,
            mode,
            out,
        } => {
            let img = lateo::codec::load(&image)?;
            let marked = match mode {
                Mode::Fragile => lateo::watermark::mark(&img, id.as_bytes(), key.as_bytes())?,
                Mode::Robust => lateo::watermark_dct::mark(&img, id.as_bytes(), key.as_bytes())?,
            };
            let out = out.unwrap_or_else(|| default_out(&image));
            lateo::codec::save_png(&out, &marked)?;
            eprintln!("lateo: marked ({:?}) id={} -> {}", mode, id, out.display());
            Ok(())
        }
        Cmd::Verify {
            image,
            key,
            mode,
            id,
        } => match mode {
            Mode::Fragile => {
                let img = lateo::codec::load(&image)?;
                match lateo::watermark::verify(&img, key.as_bytes())? {
                    Some(found) => {
                        eprintln!(
                            "lateo: fragile watermark present — id: {}",
                            String::from_utf8_lossy(&found)
                        );
                        Ok(())
                    }
                    None => Err(
                        "no fragile watermark found (wrong key, not marked, or tampered)".into(),
                    ),
                }
            }
            Mode::Robust => {
                let id = id.ok_or_else(|| {
                    "lateo: robust verify requires --id (the signature is bound to id+key)"
                        .to_string()
                })?;
                let img = lateo::codec::load(&image)?;
                let rate = lateo::watermark_dct::verify(&img, id.as_bytes(), key.as_bytes())?;
                let present = rate >= PRESENT_THRESHOLD;
                eprintln!(
                    "lateo: robust watermark — {:.0}% match ({})",
                    rate * 100.0,
                    if present { "present" } else { "absent" }
                );
                Ok(())
            }
        },
        Cmd::Scout { image } => {
            let img = lateo::codec::load(&image)?;
            let r = lateo::probe::scout(&img);
            let names = ["R", "G", "B"];
            eprintln!("lateo: scout — capacity: {} bytes", r.capacity_bytes);
            for (name, s) in names.iter().zip(r.stats.iter()) {
                eprintln!(
                    "lateo: scout {name} — χ²={:.1}, LSB-equalised pair fraction={:.3}",
                    s.chi_square, s.equalised_fraction
                );
            }
            eprintln!("lateo: scout verdict — {}", lateo::probe::scout_verdict(&r));
            Ok(())
        }
        Cmd::Probe { image, plane, out } => {
            let img = lateo::codec::load(&image)?;
            let stats = lateo::probe::analyse(&img);
            let names = ["R", "G", "B"];
            for (name, s) in names.iter().zip(stats.iter()) {
                eprintln!(
                    "lateo: probe {name} — χ²={:.1}, LSB-equalised pair fraction={:.3}",
                    s.chi_square, s.equalised_fraction
                );
            }
            eprintln!(
                "lateo: probe verdict — {}",
                lateo::probe::verdict(&stats, 0.65)
            );
            if let Some(path) = out {
                let p = lateo::probe::bit_plane(&img, plane);
                lateo::codec::save_png(&path, &p)?;
                eprintln!("lateo: wrote bit plane {plane} -> {}", path.display());
            } else {
                // default: write a bit-plane PNG next to the image, so the
                // user gets a visual artefact for free.
                let mut p = image.clone();
                p.set_extension(format!("probe.plane{plane}.png"));
                let plane_img = lateo::probe::bit_plane(&img, plane);
                lateo::codec::save_png(&p, &plane_img)?;
                eprintln!("lateo: wrote bit plane {plane} -> {}", p.display());
            }
            Ok(())
        }
    }
}

/// Read the payload from exactly one of `--message` / `--message-file`.
fn read_payload(message: Option<String>, file: Option<PathBuf>) -> lateo::Result<Vec<u8>> {
    match (message, file) {
        (Some(m), None) => Ok(m.into_bytes()),
        (None, Some(path)) => Ok(std::fs::read(&path)?),
        (Some(_), Some(_)) => {
            Err("lateo: --message and --message-file are mutually exclusive".into())
        }
        (None, None) => Err("lateo: hide requires --message or --message-file".into()),
    }
}

/// Default output path: `<image>.lateo.png` (never clobbers the input).
fn default_out(input: &Path) -> PathBuf {
    input.with_extension("lateo.png")
}
