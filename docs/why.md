---
layout: default
title: Why
---

# Why lateo?

## Two primitives, one engine

Steganography and watermarking share an underlying toolkit (image I/O, color-space transforms, frequency-domain machinery) but solve *opposite* problems:

- **Steganography** = "this image contains a payload" is the secret. You want the embed to be invisible *and* to die if anyone touches the image.
- **Watermarking** = "this image is mine" is the assertion. You want the imprint to be invisible *and* to survive a hostile transformation pipeline.

So a single "hide data" verb would either be too fragile for watermarking or too robust for steganography. The two have to be separate engines that share I/O.

## Why a new crate

There are a few mature C/C++ steganography toolkits (`steghide`, `outguess`, `openstego`) and a long history of JPEG-DCT watermarking research. lateo is not trying to outdo any of them on raw embedding quality — it's trying to make the two primitives **first-class citizens in one CLI** with:

- A single Rust binary, statically linked, ~1 MB.
- Cosign-signed release artifacts (matches the ljh-sh dist regime).
- A recipe-first docs site: every example is a real use case with honest "what this does and does *not* prove" notes.
- No network. No temp files unless asked. No surprise behaviour.

## Why Latin *lateo*

*lateo* — "I lie hidden" — is the root of *latent*. It captures both primitives at once: the message that lies hidden (steganography) and the imprint that lies hidden (watermarking). Two verbs, one word.

## Status

`lateo` is at **scaffold** stage. The CLI surface is finalised; the embedding engines are not yet implemented. See the [roadmap](https://github.com/ljh-sh/lateo#readme) for what's next.