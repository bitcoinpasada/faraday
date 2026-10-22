# The Faraday vault: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/VAULT.md` on 2026-10-22. Each keeps its
original number; the source keeps a one-line stub under the same heading.

## Header (from docs/VAULT.md)

V0.1 · 2026-10-04 · built 2026-10-05 in `faraday/faraday-vault` (format, create, open, seal, records), with the vectors in `tools/vectors/vault/` and the reference reader `tools/vault/open.py`, which opens both vectors. Not built yet: §3.1's Raspberry Pi timings, and the writer of record type 9 (`docs/history/HANDOFF.md` item 12 says why). Types 7 and 8 are written by `faraday-pgp` and `faraday-sb` through the app. Type 12 (`docs/APK-SIGNING.md`, F25) is written by `faraday-core/src/apk.rs` and read by `open.py`, with `--field TYPE.FIELD --out FILE`; the receipt of a write keeps the stick's FAT volume serial (§6).
