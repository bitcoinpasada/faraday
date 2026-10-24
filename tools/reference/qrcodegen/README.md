# Project Nayuki's QR Code generator (Python), vendored

`qrcodegen.py`, the Python edition of Project Nayuki's QR Code generator
library (MIT License, the notice in the file's head), copied unchanged
for the crypto cross-check's `seedqr` and `bbqr` areas
(`tools/crosscheck/area_seedqr.py`, `area_bbqr.py`, F79). Standard
library only.

Faraday's encoder (`core/faraday-codec/src/qr.rs`) is a port of the same
library, restructured for `no_std` and secret payloads; this file is the
library as Nayuki publishes it, so the area checks that the port still
makes the same code, module for module, at every version and level.

Source: `https://github.com/nayuki/QR-Code-generator`, commit
`3c6d0b3cefb4e049dc337e82237c9644399716a8` (master, 2026-08-31), file
`python/qrcodegen.py`. Fetched with `git fetch` of that commit, which
checks every object against the commit hash.

| File | SHA-256 |
|---|---|
| `qrcodegen.py` | `9f4ed1dd201dcb92b1bc0d6e14f46c754bcff0ce48580c5d7e8ace8f6926c8ef` |
