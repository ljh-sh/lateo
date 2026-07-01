# lateo recipes — real use cases

lateo ships four operations:

- `hide` / `extract` — steganography: hide a payload in an image, key controls *where*
- `mark` / `verify` — watermarks: an ownership imprint (two modes: **fragile** = tamper detection, **robust** = survives JPEG re-encoding)
- `probe` — steganalysis self-check: did someone hide something in *this* image?

Each recipe below is a **real, concrete use case** with copy-pasteable
commands, expected output, and an honest "what this does and does *not* prove"
note at the end. No contrived demos.

> **Setup.** `lateo` must be on your `$PATH`. Either
> `cargo install --git https://github.com/ljh-sh/lateo` (from source) or
> download a prebuilt, cosign-signed tarball from the
> [v0.1.0 release](https://github.com/ljh-sh/lateo/releases/tag/v0.1.0) and
> drop the `lateo` binary somewhere on `$PATH`. Recipes that use
> `--passphrase` additionally require a build with `--features encryption`.

---

## 1. Prove this photo is yours — robust watermark survives JPEG re-encoding

**Scenario.** You're a photographer. You publish a JPEG to your portfolio
or social media. Later, someone re-uploads it (and the platform
re-encodes it to a new JPEG, which destroys any LSB stego and any
fragile watermark). You need to prove it's still yours. The **robust**
watermark is designed to survive that — by modulating a mid-frequency
8×8-block DCT coefficient, it survives typical JPEG re-encoding at
quality ≥ 50.

```bash
# You, at publish time:
$ lateo mark --mode robust -i cover.jpg \
    --id "alice@example.com" -k "$YOUR_KEY" -o marked.jpg
lateo: marked (Robust) id=alice@example.com -> marked.jpg
```

```bash
# You, after the image has been through the wild (re-downloaded, re-JPEGed, cropped-but-not-too-much):
$ lateo verify --mode robust -i marked.jpg \
    --id "alice@example.com" -k "$YOUR_KEY"
lateo: robust watermark — 98% match (present)
```

Empirically, on a 256×256 cover, the match rate holds at ~99% after JPEG
quality 70 and ~98% after quality 50. Below quality 50 the match
degrades (the detector still prints the percentage, so you can decide
your own threshold).

> **What this does and does *not* prove.** A *positive* verify is strong
> evidence the image passed through your marking step. It does **not**
> prove *exclusive* authorship (anyone you shared the key with could
> re-mark), and it does not survive a determined attacker who has the
> original unmarked image and the freedom to subtract it. Pair with
> external provenance (C2PA, EXIF) if you need stronger claims.

---

## 2. Detect that *any* pixel was changed — fragile watermark

**Scenario.** You signed a screenshot, a contract scan, or a piece of
digital evidence. You want to know whether *any* pixel was edited after
you marked it. The **fragile** watermark breaks on the slightest
modification — exactly the property you want for tamper detection.

```bash
# You, at signing time:
$ lateo mark -i original.png \
    --id "case-2026-001" -k "$YOUR_KEY" -o signed.png
lateo: marked (Fragile) id=case-2026-001 -> signed.png
```

```bash
# You, on receipt / at audit:
$ lateo verify -i signed.png -k "$YOUR_KEY"
lateo: fragile watermark present — id: case-2026-001
```

```bash
# Anyone — even yourself — edits a pixel:
$ magick signed.png -evaluate add 1% tampered.png    # any tool that touches pixels
$ lateo verify -i tampered.png -k "$YOUR_KEY"
lateo: no fragile watermark found (wrong key, not marked, or tampered)
exit 1
```

> **What this does and does *not* prove.** A *missing* watermark proves
> the bytes have changed since marking. It does **not** prove *malice*:
> innocent operations (re-saving with a different encoder, applying a
> colour profile, cropping-then-padding) also break it. Use the fragile
> watermark as **evidence of integrity**, not as evidence of fraud. For
> "did a *specific* person edit it", you need a non-repudiable
> signature on top.

---

## 3. Send a secret message inside an image

**Scenario.** You want to send a short text (or a small file) to a
recipient, hidden inside a cover image so anyone casually looking at
the image sees only a normal photo. The recipient needs a **key** to
extract; optionally, you also want a **passphrase** so even *with* the
key, the message body is meaningless.

### 3a. Plaintext steg — key controls *where*, not *what*

```bash
# sender
$ lateo hide -i photo.png -m "meet at the bridge at midnight" \
    -k shared-key -o secret.png
lateo: hid 27 bytes (capacity 196608 bits) -> secret.png

# recipient
$ lateo extract -i secret.png -k shared-key
meet at the bridge at midnight
```

The key controls a Fisher–Yates shuffle that decides *which* pixel LSBs
hold the bits. Without the key, the envelope cannot even be located —
so without the key, the message is "hidden" in the technical sense
(Fisher–Yates spread + LSB), but in the content sense it is **plaintext**
(once extracted). For content secrecy, use 3b.

### 3b. With passphrase — content secrecy on top (AEAD)

Requires a build with `--features encryption`.

```bash
# sender
$ lateo hide -i photo.png -m "the eagle flies at midnight" \
    -k shared-key --passphrase "correct horse battery staple" \
    -o secret.png
lateo: encrypted-hid 27 bytes (capacity 196608 bits) -> secret.png

# recipient (no passphrase → clear, actionable error)
$ lateo extract -i secret.png -k shared-key
lateo: this envelope is encrypted; pass --passphrase to decrypt
exit 1

# recipient (with passphrase)
$ lateo extract -i secret.png -k shared-key \
    --passphrase "correct horse battery staple"
the eagle flies at midnight
```

Under the hood: chacha20poly1305 AEAD with a key derived from the
passphrase via argon2id (7 MiB, t=3) and a fresh per-envelope salt+nonce.
Wrong passphrase → AEAD tag mismatch → no plaintext leak. The encrypted
envelope uses a distinct magic (`LATEOSGE`) so a plaintext `extract`
on an encrypted image fails cleanly with "pass --passphrase to decrypt"
rather than returning garbage.

### 3c. Hide a binary file

```bash
# sender
$ lateo hide -i cover.png --message-file secret.bin -k key -o stego.png
lateo: hid 1024 bytes (capacity 196608 bits) -> stego.png

# recipient
$ lateo extract -i stego.png -k key > recovered.bin
```

> **What this does and does *not* prove.** `hide` makes the payload
> *unreadable* to a casual observer. It does **not** make the envelope
> *undetectable* — see recipe 4. And `--passphrase` gives content
> secrecy, but the *existence* of the envelope is still discoverable by
> someone who runs recipe 4 against your image. If you need both
> secrecy *and* deniability, you don't get it from any steganography
> tool alone — combine with a private channel (Signal, Tor, etc.).

---

## 4. "Did this image carry a hidden message?" — steganalysis self-check

**Scenario.** You received a suspicious image (from an untrusted
sender, downloaded from a sketchy site, etc.) and you want to know if
someone hid a lateo-style payload in it. `probe` runs the classical
**chi-square** LSB steganalysis (Westfeld–Pfitzmann 1999 style) per
colour channel and prints a heuristic verdict. It also writes the
chosen bit plane as a black/white PNG for visual inspection.

```bash
$ lateo probe -i suspect.png
lateo: probe R — χ²=87.4,  LSB-equalised pair fraction=0.42
lateo: probe G — χ²=112.1, LSB-equalised pair fraction=0.45
lateo: probe B — χ²=95.8,  LSB-equalised pair fraction=0.44
lateo: probe verdict — probably natural
```

On a "smooth" cover (gradient, sky, flat areas) where a lateo steg
payload is present, the **χ²** drops and the **LSB-equalised pair
fraction** rises — the verdict flips to `likely stego (LSB equalised)`.

**Visual check** — the LSB plane (default plane 0) is written next to
the image as a black/white PNG. A noisy/scrambled-looking LSB plane
suggests embedding; a structured LSB plane (one that visually echoes
the higher-bit content) is consistent with a natural image.

```bash
$ lateo probe -i suspect.png                      # default: writes suspect.probe.plane0.png
$ lateo probe -i suspect.png --plane 1            # look at bit-plane 1 instead
$ lateo probe -i suspect.png --out bitplane.png    # explicit output path
```

> **What this does and does *not* prove.** `probe` is a **heuristic**.
> Small payloads in noisy covers (textured photos, plasma, anything
> already JPEG-recompressed) stay below the radar by design — the
> equalisation effect is masked by the cover's own noise. A "probably
> natural" verdict is *not* a proof of innocence; a "likely stego"
> verdict is a reason to look closer, not a conviction. The math lives
> in `src/probe.rs`; the verdict threshold is the average
> equalised-pair fraction over the three channels (default 0.65).

---

## 5. Pick the best steg carrier — `lateo scout`

**Scenario.** You have a folder of photos and you want to send a
covert message. **Which photo should you use?** Different images have
different capacities and different "baseline" equalisation (see
recipe 4). `scout` prints both and gives a rule-based verdict, so you
can compare candidates and pick the one that (a) is large enough for
your payload and (b) leaves the most "headroom" for the embedder to
hide changes from chi-square detection.

```bash
$ lateo scout -i candidates/sky.jpg
lateo: scout — capacity: 62208 bytes
lateo: scout R — χ²=87.4,  LSB-equalised pair fraction=0.42
lateo: scout G — χ²=112.1, LSB-equalised pair fraction=0.45
lateo: scout B — χ²=95.8,  LSB-equalised pair fraction=0.44
lateo: scout verdict — good carrier: enough capacity and a baseline that leaves detection headroom
```

Run it across your candidates, then **pick the one with the highest
capacity *and* the lowest average equalised pair fraction**.

> **What this does and does *not* prove.** Scout gives you a *heuristic*
> ranking, not a guarantee. The numbers it prints are the *baseline*
> stats of the unmodified cover; embedding will change them, and the
> `probe` detector (recipe 4) measures the change. A "good carrier"
> verdict means "this image is large and its baseline is not already
> near the detector's threshold" — it does **not** mean embedding is
> invisible. For genuine anti-detection you need adaptive embedding
> (J-UNIWARD, HUGO, …) which lateo does not implement.

## What's next

- **Carrier-quality / anti-forensic** features (adaptive embedding,
  RS-analysis) — gated behind a cargo feature so the default build
  stays zero-cost.
