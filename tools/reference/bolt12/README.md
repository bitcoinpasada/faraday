# Core Lightning's offer decoder, for BOLT #12

The crypto cross-check's `lightning` area (`tools/crosscheck/area_lightning.py`)
asks Core Lightning's own offer decoder what it makes of a BOLT #12 offer and
compares the answer with Faraday's reader (`core/faraday-codec/src/bolt12.rs`).
The decoder is `devtools/bolt12-cli decode` of Core Lightning, built from its
source release; nothing of it is copied into this repository.

| What | Value |
|---|---|
| Project | Core Lightning, <https://github.com/ElementsProject/lightning> (MIT) |
| Tag | `v26.06.10`, commit `e8b1609e67575036bd80aec32605e830f8f651cc` |
| Archive | the release's `clightning-v26.06.10.zip` (with the libwally-core and libbacktrace submodules), SHA-256 `4914561e6d108dacca0183d7e6b3b1c224c580597ddcbb397299ef5b0fcc3230`, the line of the release's own `SHA256SUMS` |
| Pin | `release.pin` in this folder |

`tools/reference/cln/` takes its hsm_secret code from the same tag: both
judges are one release of Core Lightning.

## How it is built

`python3 -I -B tools/crosscheck/fetch_cln.py` fetches the archive once into
`~/.cache/faraday/core-lightning` (or `FARADAY_CLN_SOURCE`) and checks its
hash against `release.pin`. `tools/crosscheck/run.sh` then calls
`tools/crosscheck/build_bolt12_cli.sh out/crosscheck`, which checks the hash
again, unpacks the archive and runs Core Lightning's own `./configure
--disable-rust --disable-valgrind` and `make devtools/bolt12-cli`, with the
version set to the tag's (the archive has no git history). The result is
`out/crosscheck/bolt12-cli`. Offline; the packages the build needs are in
`docs/DEVELOPING.md` §1.

## What is left out, and what the area does around it

- Only `bolt12-cli` is built: not `lightningd`, the plugins or the Rust parts.
- `bolt12-cli decode` passes the decoder no feature set, so an offer that sets
  a feature bit is read; Faraday refuses an unknown required one (F232), which
  the area names as a stated difference.
- The command reads one Bech32 string. BOLT #12's `+` joins and a
  `lightning:` prefix are for its callers, so the area takes out a `+` between
  two Bech32 characters (and the white space after it) and the prefix before
  asking it; a `+` anywhere else is passed on and refused by the command.
- The command prints the fields as text; `area_lightning.py` reads that text
  into the shape Faraday answers in.
