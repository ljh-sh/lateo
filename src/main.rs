//! lateo CLI — imperceptible data embedding (steganography + watermarking).
//!
//! Subcommands dispatch to the engines in `lateo`:
//! - `hide` / `extract` → [`lateo::steg`] (LSB keyed-spread, fragile)
//! - `mark` / `verify --mode fragile` → [`lateo::watermark`] (tamper detection)
//! - `mark` / `verify --mode robust`  → [`lateo::watermark_dct`] (survives mild JPEG)

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
    /// Reverse self-check: can the embedding be detected / stripped? (stub)
    Probe {},
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

fn run(cli: Cli) -> lateo::Result<()> {
    match cli.cmd {
        Cmd::Hide {
            image,
            key,
            message,
            message_file,
            out,
        } => {
            let payload = read_payload(message, message_file)?;
            let img = lateo::codec::load(&image)?;
            let stego = lateo::steg::hide(&img, &payload, key.as_bytes())?;
            let out = out.unwrap_or_else(|| default_out(&image));
            lateo::codec::save_png(&out, &stego)?;
            eprintln!(
                "lateo: hid {} bytes (capacity {} bits) -> {}",
                payload.len(),
                lateo::lsb::capacity_bits(&img),
                out.display()
            );
            Ok(())
        }
        Cmd::Extract { image, key } => {
            let img = lateo::codec::load(&image)?;
            match lateo::steg::extract(&img, key.as_bytes())? {
                Some(payload) => {
                    let mut out = std::io::stdout().lock();
                    out.write_all(&payload)?;
                    out.flush()?;
                    Ok(())
                }
                None => Err("no payload found (wrong key, not a stego image, or tampered)".into()),
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
        Cmd::Probe {} => {
            eprintln!("lateo: probe (steganalysis self-check) is not implemented yet");
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
