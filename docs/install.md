---
layout: default
title: Install
---

# Install lateo

`lateo` ships two install paths.

## Prebuilt binary (cosign-signed)

Five targets: Linux x86_64 / arm64, macOS x86_64 / arm64, Windows x86_64.

```sh
# pick the tarball for your platform, then:
tar xJf lateo-<target>.tar.xz -C /usr/local/bin --strip-components=1 bin/lateo
```

Verify:

```sh
sha256sum -c SHA256SUMS --ignore-missing
cosign verify-blob --bundle lateo-<target>.tar.xz.sigstore.json lateo-<target>.tar.xz
```

See the [release page](https://github.com/ljh-sh/lateo/releases/latest) for the matching artifact.

## From source

```sh
cargo install --git https://github.com/ljh-sh/lateo
```

For the `--features encryption` build (passphrase-protected payloads):

```sh
cargo install --git https://github.com/ljh-sh/lateo --features encryption
```

> **Note.** The `lateo` binary is at `~/.cargo/bin/lateo` after install. Make sure that's on your `$PATH` for the [recipes]({{ '/recipes' | relative_url }}) to work.