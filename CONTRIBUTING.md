# Contributing to lateo

Thanks for your interest! lateo is a focused image data-embedding toolkit.
Please read this short guide before opening an issue or PR.

## Scope

lateo does two things, behind one CLI:

- **steganography** (`hide` / `extract`) — a covert, fragile, high-capacity payload;
- **watermarking** (`mark` / `verify`) — a robust, low-capacity ownership imprint.

The two are separate engines (opposed optimisation targets) sharing image I/O
and transform plumbing. If your idea blurs them into one "hide data" path, open
an issue first — the split is intentional.

## Reporting issues

Open a [GitHub issue](../../issues) and include:

- Operating system and architecture
- lateo version (`lateo --version`)
- Installation method (cargo / binary / source)
- The exact command you ran and a minimal input image
- Expected vs actual output

## Building from source

```bash
cargo build --release
# binary at target/release/lateo
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## Pull requests

- Keep `#![forbid(unsafe_code)]`. Image decode paths must return `Result`, not
  panic, on malformed input.
- Run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`
  before pushing.
- Do not use auto-close keywords (`Closes`, `Fixes`, `Resolves`) in commit
  messages or PR descriptions. Link issues by number in prose instead.
