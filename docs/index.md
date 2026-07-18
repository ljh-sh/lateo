---
layout: default
title: Home
---

<div class="hero">
  <h1>lateo</h1>
  <p>Imperceptible data embedding for images — steganography (a covert <em>message</em>) and watermarking (a robust <em>imprint</em>), sharing one engine.</p>
  <div class="cta">
    <a class="btn primary" href="{{ '/install' | relative_url }}">Install</a>
    <a class="btn secondary" href="{{ '/cli' | relative_url }}">CLI verbs</a>
    <a class="btn secondary" href="{{ '/recipes' | relative_url }}">Recipes</a>
  </div>
</div>

<div class="badges">
  <a href="https://github.com/ljh-sh/lateo/blob/main/LICENSE" title="Apache 2.0"><img alt="License" src="https://img.shields.io/badge/license-Apache_2.0-blue.svg"></a>
  <a href="https://scorecard.dev/viewer/?uri=github.com/ljh-sh/lateo" title="OpenSSF Scorecard"><img alt="OpenSSF Scorecard" src="https://api.securityscorecards.dev/projects/github.com/ljh-sh/lateo/badge"></a>
  <a href="https://github.com/ljh-sh/lateo/actions" title="CI"><img alt="Build status" src="https://img.shields.io/github/actions/workflow-status/ljh-sh/lateo/ci.yml?branch=main&amp;logo=github-actions&amp;logoColor=white"></a>
</div>

## What is lateo?

**lateo** (Latin *lateo* — "I lie hidden") is a Rust toolkit for **imperceptible** data embedding in images. Two distinct primitives share one engine:

| | steganography | watermarking |
| --- | --- | --- |
| what's hidden | a covert **message** | a robust **imprint** |
| existence | **denied** (nobody knows) | **asserted** (you announce it) |
| robustness | fragile (dies on re-encode) | survives re-encoding |
| capacity | high | low |

They are deliberately separate verbs, not one "hide data" call — their optimisation targets are opposed (imperceptibility-and-capacity vs. robustness), so they need distinct engines even though they share image I/O and transform plumbing.

## Five verbs, one engine

```sh
lateo hide    image.png  -m 'a secret'    # embed a covert payload (fragile)
lateo extract image.png                  # recover it
lateo mark    image.png  -i owner-id      # embed a robust imprint
lateo verify  image.png                  # check the imprint survived
lateo probe   image.png                  # can this be detected / stripped?
```

> **Status: scaffold.** The verbs above are wired but the embedding engines are not yet implemented. `lateo <verb>` today reports "not implemented".

See [CLI verbs]({{ '/cli' | relative_url }}) for the verb reference, or jump straight to [Recipes]({{ '/recipes' | relative_url }}) for real use cases with copy-pasteable commands.

## Why one engine for two primitives

Steganography and watermarking look like siblings but solve different problems:

- **Steganography** = "this contains a message" is the secret. Imperceptibility + capacity are the levers. Robustness is *anti-feature*: if re-encoding destroys the message, that's the point.
- **Watermarking** = "this is *my* image" is the assertion. Robustness is the lever. Imperceptibility and capacity are *anti-features*: you don't need much payload, and you don't want anyone to find it.

So the shared parts (image I/O, color-space transforms, the cosine/DCT machinery, the encode/decode round-trip) live in one crate. The two engines sit on top and tune the same primitives in opposite directions. See [Why lateo]({{ '/why' | relative_url }}) for the design notes.