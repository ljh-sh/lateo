# lateo

[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/ljh-sh/lateo/badge)](https://scorecard.dev/viewer/?uri=github.com/ljh-sh/lateo)

> Imperceptible data embedding for images — steganography (a covert *message*)
> and watermarking (a robust *imprint*), sharing one engine.

> 图像的不可见数据嵌入 —— 隐写(藏起来的*消息*)与水印(藏起来的*烙印*),共用一个引擎。 — [中文文档](README.cn.md)

## TL;DR

```bash
lateo hide    image.png  -m secret.txt   # embed a covert payload (fragile)
lateo extract image.png                  # recover it
lateo mark    image.png  -i owner-id      # embed a robust imprint
lateo verify  image.png                  # check the imprint survived
lateo probe   image.png                  # can this be detected / stripped?
```

> **Status: scaffold.** The verbs above are wired but the embedding engines
> are not yet implemented. `lateo <verb>` today reports "not implemented".

## What this is

`lateo` puts two information-hiding primitives behind one CLI:

| | steganography | watermarking |
| --- | --- | --- |
| what's hidden | a covert **message** | a robust **imprint** |
| existence | **denied** (nobody knows) | **asserted** (you announce it) |
| robustness | fragile (dies on re-encode) | survives re-encoding |
| capacity | high | low |

They are deliberately separate verbs, not one "hide data" call: their
optimisation targets are opposed (imperceptibility-and-capacity vs. robustness),
so they need distinct engines even though they share the image I/O and
transform plumbing.

The name is Latin ***lateo*** — *"I lie hidden"* — the root of *latent*.

## Install

**From source** (full feature set):

```bash
cargo install --git https://github.com/ljh-sh/lateo
```

Prebuilt, cosign-signed binaries (Linux x86_64/arm64, macOS x86_64/arm64,
Windows x86_64) will attach to the first release.

## License

Apache-2.0.
