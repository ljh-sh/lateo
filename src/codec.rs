//! Image I/O for the lateo engines.
//!
//! v1 reads/writes **lossless PNG** (via the `image` crate with only the `png`
//! feature). Embedding into lossy JPEG is out of scope: re-encoding a JPEG
//! destroys LSB data (see `SECURITY.md`). `image::open` will still decode other
//! formats it recognises, but only PNG round-trips losslessly through `save`.

use image::{ImageReader, RgbaImage};
use std::path::Path;

/// Load an image from disk as an RGBA8 buffer (the carrier the engines work on).
pub fn load(path: &Path) -> crate::Result<RgbaImage> {
    let img = ImageReader::open(path)?.with_guessed_format()?.decode()?;
    Ok(img.to_rgba8())
}

/// Write an RGBA8 buffer to disk as PNG.
pub fn save_png(path: &Path, img: &RgbaImage) -> crate::Result<()> {
    img.save(path)?;
    Ok(())
}
