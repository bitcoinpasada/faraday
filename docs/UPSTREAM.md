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
- `core/osk-bip/src/json.rs`, `osk_bip::json` (2026-10-20): Faraday's
  JSON reader (RFC 8259, every escape, at most 32 deep and 1 MiB, a key
  named twice refused) moved in from `faraday-intake`, its fuzz target
  into `fuzz/` as `json`. `osk_bip::coldcard` reads its file with it
  instead of a scanner that took the first `"name"` anywhere in the
  text and undid no escapes, and `core_import` writes the descriptor
  with `json::quote`. Why: one JSON reader for every wallet file, in the
  crate that reads the formats. Stricter only: text that is not JSON,
  names a key twice or nests deeper than 32 is no Coldcard export.
- `core/osk-bip/src/slip132.rs` (2026-10-20): one decoder,
  `slip132::decode`, for SLIP-0132's ten public versions, multisig
  `Ypub`/`Zpub`/`Upub`/`Vpub` included, saying which script a version
  names (none for `xpub`/`tpub`, which are not Legacy); with it the
  agreement check (`agrees`, `Mismatch { key, named, around }`),
  `plain_descriptor`, `key_line` and `encode`, moved in from
  `faraday-intake` (the refusal's sentence stays there). `decode_xpub`
  keeps its contract on top of it. `multisig_config` and `coldcard` read
  their keys with it instead of their own unchecked version tables. Why:
  the two core readers converted a SLIP-132 key whatever script it
  named, so a config with a `zpub` or a `Ypub` in a P2WSH wallet loaded
  wherever Faraday's own check was not run first (a rebuilt backup
  config, `restore::readable`). Stricter only: such a key is refused
  (`policy::Error::Key`; `multisig_config::slip132_mismatch` names it).

## 4. Watching upstream

When there is reason to (a release, a security advisory, or every few
weeks), fetch `upstream` and read `git log d26b84e..upstream/main`. Port
what matters, security fixes first, as a commit per upstream change
naming upstream's commit, and move §1's sync point. Notes once meant for
upstream (`local/for-upstream/`) are optional: they may still be sent,
and nothing waits on them.
