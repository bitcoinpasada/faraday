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
- `core/osk-bip/src/coldcard.rs` (2026-10-20): one reader of Coldcard's
  files. `read` (and `parse` on it) roots every key at the top-level
  `xfp`, the master's, where `parse` took the `xfp` inside the account,
  which is the account key's own and matches no seed; it checks the
  account's stated `first` address against the wallet's, refuses an
  account with a key and no `deriv` (or an empty one, which would claim
  the account key is the master), and returns why (`ReadError`).
  `cosigner_key` checks each key's SLIP-132 version against its field;
  `cosigner_keys`, Faraday's list (the flat `p2wsh`, `p2sh_p2wsh`,
  `p2wsh_p2sh` and `p2sh`, then `bip48_2`, `bip48_1`, `bip45`, `bip86`,
  `bip84`, `bip49`, `bip44`), moved in from `faraday-core`'s
  `create::coldcard_keys`, and `faraday-intake`'s own reader is gone.
  Why: upstream's reader built a wallet no seed signs for. Changes for
  OpenSigner's scan of a real Coldcard file: the wallet is the master's,
  not refused or mis-rooted; nothing it read correctly before reads
  differently (its tests' file repeats the master's `xfp` per account).
  Its test export's two `first` addresses were no key's; they are now
  the keys' own.
- `core/osk-bip/src/bsms.rs` (2026-10-20): `DescriptorRecord::parse`
  compares the wallet's first address with the stated one as its
  encoding writes it (`assume_checked().to_string()`; the network check
  stays), so an all upper-case bech32 address, which BIP 173 has a
  reader take, is read; and a `/**` template whose paths line is `No path
  restrictions` reads as `/0/*,/1/*` (F64, the owner's decision; the
  first-address check still decides). `faraday-intake`'s tidy-up before
  the parser (`bsms_as_upstream_reads`) is gone. Why: Sparrow writes and
  reads both. Accepts more than before only in these two owner-confirmed
  cases; a base58 address in another case and every record Sparrow
  refuses stay refused.
- `core/osk-bip/src/core_import.rs` (2026-10-20):
  `import_descriptors_pair`, with `pair_descriptors` and `writes_pair`,
  writes Bitcoin Core's `importdescriptors` file for Core before 29 (a
  receive and a change request, each with its own checksum, the string
  written with `json::quote`), moved in from `faraday-intake`'s
  `core_before_29`. Why: the format's writer beside the multipath one.
  Same output as before.
- `core/osk-codec/src/ur/mod.rs` (2026-10-20): `ur::encode_single(ur_type,
  cbor)` is public (it was private, arguments the other way round), so
  Faraday writes `ur:crypto-account` with it; `faraday-qr`'s copy of the
  bytewords table and CRC-32 (`faraday-qr/src/ur.rs`) is gone. Why: one
  copy of the alphabet and its checksum. Same output as before.
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
- `core/osk-codec/src/decode.rs` (2026-10-21; `docs/SPEED.md` §9):
  `decode::decode_picture` reads a still picture for codes dark on
  light and, when it holds none, light on dark, counted once;
  `decode_luma` and it share the grid reading. Why: Faraday's PNG
  reader inverted the picture into a second copy and counted two
  decodes. Same codes read as before.
- `core/osk-ui/src/canvas.rs` (2026-10-21; `docs/SPEED.md` §9), five
  changes: a line of text wholly above or below
  the clip is measured, not drawn, and `Canvas::lines_drawn` counts the
  lines drawn; `shadow` skips its covered middle; `blend_from` passes
  over runs where the two frames agree; `restore` of the same pixels
  again copies only the box written since (`damaged`, called by every
  drawing call); `frost` on a dense screen (a reduced pixel 16 or more
  across) reads every other pixel of every other row and enlarges per
  two by two pixels. Why: frames a person waits on at HiDPI. The first
  four draw the same pixels; the fifth differs by at most 3 levels in
  a frosted backdrop at scale 2 and above.
- `core/osk-bip/src/keys.rs` (2026-10-21; `docs/COMPUTE.md` §5.1):
  `MasterKey` keeps its fingerprint, worked out once when the key is
  made, in its blinded context; `for_network` copies it, `fingerprint()`
  returns it, and `derive()` takes the derived key's master fingerprint
  from it. Why: one elliptic-curve multiplication saved per fingerprint
  asked and per derivation, in both apps; Faraday's `wallet::Key` no
  longer keeps its own copy. Same output as before; the BIP-32 vectors
  pass unchanged.
- `core/osk-bip/src/policy.rs` (2026-10-21; `docs/COMPUTE.md` §5.2):
  `WalletPolicy` keeps its descriptor checksum, worked out once by every
  constructor; `checksum()` returns it as `&str` (it returned a new
  `String`), and `opensigner-core`'s four callers copy it where they
  keep it. Why: the checksum was written out and hashed again on every
  ask, drawing included; Faraday's `wallet::Wallet` no longer keeps its
  own copy. Same output as before.
- `core/osk-bip` (2026-10-21; `docs/COMPUTE.md` §5.3, F66): hardened
  steps are kept and written `h`. Reading is unchanged (`'`, `h`, mixed);
  `policy.rs` keeps a key's origin written `h` and writes a miniscript
  wallet's descriptor with `h`; `recovery.rs` writes a recovery path's
  keys the same way. `bsms.rs`: a received key record keeps its key line
  as received and its signature is checked over that line
  (`SignerRecord::key_line`); a record made here writes `h`; BIP 129's
  own records still read and verify, and its descriptor records are
  written back with `h`. The PSBT fixtures' wallets
  (`tools/vectors/psbt/*.policy`, the two cosigner lines) are
  regenerated with `h`. Why: one wallet has one text and one checksum
  however it was written, so the checksum can be kept on the policy,
  and Faraday's second parse (`faraday_intake::wallet::hardened_h`) is
  gone. For OpenSigner's app: wallets it made already wrote `h`; a
  wallet imported with `'` now shows and exports `h` and a different
  checksum, and `osk-keep`'s names and kept-wallet choices keyed by the
  old checksum are lost for it.

## 4. Watching upstream

When there is reason to (a release, a security advisory, or every few
weeks), fetch `upstream` and read `git log d26b84e..upstream/main`. Port
what matters, security fixes first, as a commit per upstream change
naming upstream's commit, and move §1's sync point. Notes once meant for
upstream (`local/for-upstream/`) are optional: they may still be sent,
and nothing waits on them.
- `core/osk-crypto/src/work.rs` (2026-10-21; `docs/COMPUTE.md` C0):
  `Compute`, the token costly work will take: not `Clone`, `Copy`,
  `Default`, `Send` or `Sync`; minted by `Compute::new(count)` at an
  application's entry points, or by `Compute::unchecked()` behind the new
  `unchecked` feature. Nothing takes it yet. Why: a screen that does
  costly work while drawing will not compile (C1-C3).
- `opensigner/opensigner-core` (2026-10-21; C0): enables
  `osk-crypto/unchecked` and has a crate-private `unchecked()` helper,
  never re-exported, for OpenSigner's own app, which makes no drawing
  claim. Faraday's crates are kept off it by `faraday/clippy.toml` and,
  from D1, the costs lint.
