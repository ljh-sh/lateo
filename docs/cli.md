---
layout: default
title: CLI
---

# CLI verbs

`lateo` exposes five verbs, two pairs plus one diagnostic:

| verb | what it does |
|---|---|
| `hide` | embed a covert payload into an image (fragile — dies on re-encode) |
| `extract` | recover the payload from an image |
| `mark` | embed a robust ownership imprint (survives JPEG re-encoding) |
| `verify` | check whether the imprint is still present |
| `probe` | steganalysis self-check — can the embed be detected / stripped? |

> **Status.** The verb surface is wired but the embedding engines are not yet implemented. `lateo <verb>` today reports "not implemented".

## `hide`

```sh
lateo hide image.png -m 'a secret'
# or: lateo hide image.png --message-file msg.txt
# or: lateo hide image.png -m 'secret' --passphrase hunter2   # requires --features encryption
```

Embeds a fragile payload. The cover image is overwritten in place; pass `--out` to write elsewhere. The payload is destroyed by any lossy re-encode (JPEG, resize, recompression).

## `extract`

```sh
lateo extract image.png
# or: lateo extract image.png -o recovered.txt
```

Recovers the payload hidden by `hide`. Returns non-zero with a clear error if no payload is present.

## `mark`

```sh
lateo mark image.png -i owner-id
# or: lateo mark image.png -i owner-id --mode robust   # default: robust
# fragile mode (tamper detection): lateo mark image.png -i owner-id --mode fragile
```

Embeds an ownership imprint designed to survive JPEG re-encoding at quality ≥ 50. Two modes:

- `robust` (default): survives re-encoding. Use for copyright / provenance.
- `fragile`: dies on any modification. Use for tamper detection.

## `verify`

```sh
lateo verify image.png
# or: lateo verify image.png --strict   # exit non-zero if imprint missing
```

Checks whether the imprint embedded by `mark` is still recoverable. With `--strict`, exits non-zero on miss — useful for CI / automation.

## `probe`

```sh
lateo probe image.png
# or: lateo probe image.png --json   # machine-readable output
```

Runs a steganalysis self-check. Useful for two things:

1. **Before publishing.** Did you accidentally embed something sensitive in an image? (Tooling sometimes does this silently.)
2. **During a forensics investigation.** Does this image carry an embed, and is the embed removable?

`--json` emits a structured report for piping into other tools.

## Global flags

| flag | meaning |
|---|---|
| `--out PATH` | write result to PATH instead of overwriting in place |
| `--passphrase S` | derive the key from S instead of a fresh random (requires `--features encryption`) |
| `--json` | machine-readable output (where applicable) |
| `--quiet` | suppress progress output |