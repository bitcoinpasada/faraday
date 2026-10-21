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
- `core/osk-shell-api`: keys and events Faraday's desktop shells send that
  upstream's enums do not have: `Key::Delete`, `Event::Wheel`,
  `ScrollEnd`, `Shift`, `Back`, `Forward`, `Hover` and `HoverEnd` (since
  0.1.1), and `Key::Home`, `End` and `SelectAll` (Ctrl+A) for the
  Starter's fields (2026-10-13). Each is optional: a core that does not
  know one ignores it, and a shell that has no such key never sends it.

Since the fork:

- `docs/learn/*.md` and `osk-learn`'s `en.rs` (2026-10-20): the Learn
  pages Faraday shows name Faraday and its screens instead of
  OpenSigner's: start here, words, transactions, randomness, where
  randomness comes from, backups, Seed XOR, passphrases, verifying, the
  air gap, scams, multisig, spend paths, xpubs, nonces (Low R and
  deterministic Schnorr with no setting), signing a message, the
  glossary (PIN and the glyph entries out, Vault and Watch-only in),
  tools, kinds of wallets, backups in other forms, coordinator files and
  silent payments. Bitcoin facts unchanged but for two additions: a die
  read as a coin (randomness) and a derivation path's hardened mark
  written `h` or an apostrophe (glossary). "Devices", "Encrypted
  backups" and "The secure element" are unchanged, waiting on the
  owner.
- The count of costly work (2026-10-21; `docs/SPEED.md` §8, F65's
  compute token): `core/osk-crypto` gains `work` (`Work`,
  `set_counter`, `count`), empty unless its new `count` feature is on,
  which needs `std`; Faraday's app core turns it on and installs its
  counter. Each costly function counts itself on its first line:
  `osk-psbt`'s `Psbt::parse_bytes` past the magic check (a PSBT
  parsed), `inspect` (inspected), `sign::sign` and `message::sign`
  (signed); `osk-codec`'s `decode::decode_luma` (QR codes looked for);
  `osk-crypto`'s `pbkdf2_hmac_sha256`, `pbkdf2_hmac_sha512` and
  `scrypt` (a key stretched; scrypt's own two PBKDF2 passes go through
  an uncounted helper, so it counts once); `osk-backup`'s `argon2id`;
  `osk-ui`'s `Canvas::frost` (blurred) and `snapshot`, `snapshot_into`,
  `restore`, `blend_from` (a whole canvas copied), with `osk-ui` now
  depending on `osk-crypto`. Nothing computed changes.

## 4. Watching upstream

When there is reason to (a release, a security advisory, or every few
weeks), fetch `upstream` and read `git log d26b84e..upstream/main`. Port
what matters, security fixes first, as a commit per upstream change
naming upstream's commit, and move §1's sync point. Notes once meant for
upstream (`local/for-upstream/`) are optional: they may still be sent,
and nothing waits on them.
