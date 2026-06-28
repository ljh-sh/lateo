# Security Policy

## Scope

This document describes the security properties of `lateo` — a Rust toolkit for
imperceptible data embedding in images (steganography and watermarking).

## Threat model

`lateo` reads and writes image files on the local filesystem. It performs no
network access and (at present) links only to the Rust standard library. As
the embedding engines land, the dependency surface will grow to image codec
crates; this section will be updated then.

The primary input trust boundary is **untrusted image files**. Decoders are a
historically rich source of panics, integer overflows, and memory-safety bugs.
`lateo` is `#![forbid(unsafe_code)]`; image decode errors must surface as
`Result`s, never as panics on attacker-controlled input.

## What steganography / watermarking are NOT

`hide` and `mark` are **not cryptographic security mechanisms**. They reduce
detectability and resist casual removal; they do **not** guarantee secrecy
against a determined, informed adversary:

- a covert payload can be **detected** by statistical steganalysis, and
- either embedding can be **stripped** by re-encoding the image.

For secrecy of *content*, combine `hide` with encryption (encrypt-then-embed).
See the README for the imperceptibility ↔ robustness trade-off.

## Reporting a vulnerability

Please open a private security advisory:
**<https://github.com/ljh-sh/lateo/security/advisories/new>**.

Do not open a public issue for suspected security problems.
