# Faraday and OpenSigner

**Status:** 2026-10-20 · Faraday is a soft fork of OpenSignerKit/OpenSigner
(`docs/DECISIONS.md` F65).

## 1. Where Faraday forked

`core/` and `opensigner/` came from OpenSignerKit (the git remote
`upstream`, `maxmoney21m/opensignerkit`). The last sync was upstream's
`d26b84e` (§16.141, 2026-10-10). Upstream had no newer commit when
Faraday took the code in house on 2026-10-20.

## 2. The rules

- Faraday may change any file in `core/` and `opensigner/`.
- Each such change is its own commit, touching nothing outside those two
  folders but its callers and tests, and its message says what changed
  and why. It is listed in §3 the same day.
- A change to cryptography (key derivation, mnemonics, signatures,
  PSBTs, message signatures, the entropy and secret-handling crates) is
  tested against published vectors or an independent implementation
  before it is committed, as everything there already is.
- Conservative as OpenSigner is: no change loosens a check. What it
  refuses stays refused (a multisig key without an origin, a guessed
  path, an ambiguous input); accepting more than a standard requires is
  the owner's decision (F65).
- OpenSigner's licence and copyright notices stay as they are.
- Faraday's strings stay in `faraday/faraday-strings`; OpenSigner's
  strings file keeps OpenSigner's strings (CLAUDE.md).

## 3. Faraday's changes since `d26b84e`

Carried before the fork (`HANDOFF.md`, Upstream, has the detail):

- `core/osk-bip/src/silent.rs`: `looks_like_address` and
  `decode_address` read the first bytes with `str::get`, so text whose
  first characters are not ASCII is not taken for an address
  (2026-10-12).
- Constant-time secret multiplication: `musig.rs`, `frost.rs`
  (`secret_base_mul`), `silent.rs` (`secp256k1::ecdh`), `osk-psbt`'s
  `sign_threshold`, with their callers (2026-10-13; tracker #78).
- `core/osk-codec`'s `png::grey_png`; `core/osk-ui`'s `Icon::SdCard` and
  `Icon::Add` (2026-10-19; tracker #64).
- `docs/learn/06-where-randomness.md` and `osk-learn`'s `en.rs`: the
  dice section's sentence on 70 and 140 rolls.

Since the fork: none yet.

## 4. Watching upstream

When there is reason to (a release, a security advisory, or every few
weeks), fetch `upstream` and read `git log d26b84e..upstream/main`. Port
what matters, security fixes first, as a commit per upstream change
naming upstream's commit, and move §1's sync point. Notes once meant for
upstream (`local/for-upstream/`) are optional: they may still be sent,
and nothing waits on them.
